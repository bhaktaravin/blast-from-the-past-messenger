use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use futures_util::{SinkExt, StreamExt};
use rand::rngs::OsRng;
use sqlx::{PgPool, Row};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use chatmessagediscordclone::protocol::{
    ClientToServer, HistoryTarget, MessageRecord, ServerToClient, UserStatus,
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone)]
struct Peer {
    user_id: Option<i64>,
    username: String,
    away: Option<String>,
    tx: mpsc::UnboundedSender<Message>,
}

#[derive(Clone, Copy)]
struct RateState {
    window_start: Instant,
    count: u32,
}

#[tokio::main]
async fn main() {
    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:9001".to_string());
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("SUPABASE_DB_URL"))
        .or_else(|_| std::env::var("SUPABASE_URL"))
        .expect("DATABASE_URL, SUPABASE_DB_URL, or SUPABASE_URL is required");
    let db = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");
    init_db(&db).await.expect("failed to init database");

    let listener = TcpListener::bind(&addr)
        .await
        .expect("failed to bind address");

    println!("AOL-style chat server running on ws://{addr}");

    let peers: Arc<Mutex<HashMap<usize, Peer>>> = Arc::new(Mutex::new(HashMap::new()));
    let rate_limits: Arc<Mutex<HashMap<i64, RateState>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("accept error: {err}");
                continue;
            }
        };

        let peer_map = Arc::clone(&peers);
        let db = db.clone();
        let rate_limits = Arc::clone(&rate_limits);
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, peer_map, db, rate_limits).await {
                eprintln!("connection error: {err}");
            }
        });
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    peers: Arc<Mutex<HashMap<usize, Peer>>>,
    db: PgPool,
    rate_limits: Arc<Mutex<HashMap<i64, RateState>>>,
) -> Result<(), String> {
    let ws_stream = accept_async(stream)
        .await
        .map_err(|err| err.to_string())?;
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();
    let writer = tokio::spawn(async move {
        while let Some(message) = out_rx.recv().await {
            if ws_tx.send(message).await.is_err() {
                break;
            }
        }
    });

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let guest_name = format!("Guest{id}");

    {
        let mut guard = peers.lock().map_err(|_| "peer lock poisoned")?;
        guard.insert(
            id,
            Peer {
                user_id: None,
                username: guest_name,
                away: None,
                tx: out_tx.clone(),
            },
        );
    }

    send_to_all(
        &peers,
        ServerToClient::Welcome {
            message: "Welcome to the Retro Chat Server".to_string(),
        },
    );
    send_to_peer(
        &peers,
        id,
        ServerToClient::System {
            message: "Please log in or register to continue.".to_string(),
        },
    );

    while let Some(message) = ws_rx.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Ok(event) = serde_json::from_str::<ClientToServer>(&text) {
                    handle_client_event(id, event, &peers, &db, &rate_limits).await;
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }

    {
        let mut guard = peers.lock().map_err(|_| "peer lock poisoned")?;
        guard.remove(&id);
    }

    broadcast_presence(&peers);
    writer.abort();

    Ok(())
}

async fn handle_client_event(
    id: usize,
    event: ClientToServer,
    peers: &Arc<Mutex<HashMap<usize, Peer>>>,
    db: &PgPool,
    rate_limits: &Arc<Mutex<HashMap<i64, RateState>>>,
) {
    match event {
        ClientToServer::Register { username, password } => {
            match create_user(db, &username, &password).await {
                Ok(user_id) => {
                    set_peer_auth(peers, id, user_id, username.clone());
                    send_to_peer(peers, id, ServerToClient::AuthOk { username });
                    broadcast_presence(peers);
                    send_threads_to_peer(db, peers, id, user_id).await;
                }
                Err(message) => {
                    send_to_peer(peers, id, ServerToClient::AuthError { message });
                }
            }
        }
        ClientToServer::Login { username, password } => {
            match verify_user(db, &username, &password).await {
                Ok(user_id) => {
                    set_peer_auth(peers, id, user_id, username.clone());
                    send_to_peer(peers, id, ServerToClient::AuthOk { username });
                    broadcast_presence(peers);
                    send_threads_to_peer(db, peers, id, user_id).await;
                }
                Err(message) => {
                    send_to_peer(peers, id, ServerToClient::AuthError { message });
                }
            }
        }
        ClientToServer::SetAway { away } => {
            if !is_authed(peers, id) {
                send_to_peer(
                    peers,
                    id,
                    ServerToClient::AuthError {
                        message: "Please log in first.".to_string(),
                    },
                );
                return;
            }
            if let Ok(mut guard) = peers.lock() {
                if let Some(peer) = guard.get_mut(&id) {
                    peer.away = away;
                }
            }
            broadcast_presence(peers);
        }
        ClientToServer::Chat { body } => {
            let (from, user_id) = match get_peer_identity(peers, id) {
                Some(info) => info,
                None => {
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::AuthError {
                            message: "Please log in first.".to_string(),
                        },
                    );
                    return;
                }
            };
            if !allow_rate(rate_limits, user_id) {
                send_to_peer(
                    peers,
                    id,
                    ServerToClient::System {
                        message: "Rate limit exceeded.".to_string(),
                    },
                );
                return;
            }
            let _ = insert_message(db, user_id, None, &body).await;
            send_chat_to_all(db, peers, user_id, &from, &body).await;
        }
        ClientToServer::DirectMessage { to, body } => {
            let (from, user_id) = match get_peer_identity(peers, id) {
                Some(info) => info,
                None => {
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::AuthError {
                            message: "Please log in first.".to_string(),
                        },
                    );
                    return;
                }
            };
            if !allow_rate(rate_limits, user_id) {
                send_to_peer(
                    peers,
                    id,
                    ServerToClient::System {
                        message: "Rate limit exceeded.".to_string(),
                    },
                );
                return;
            }
            if let Ok(Some(target_id)) = get_user_id_by_name(db, &to).await {
                if is_blocked_or_muted(db, target_id, user_id).await {
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::System {
                            message: "User is not accepting messages.".to_string(),
                        },
                    );
                    return;
                }
                let _ = insert_message(db, user_id, Some(target_id), &body).await;
                let delivered = send_dm_to_user(peers, target_id, &from, &body);
                send_threads_to_peer(db, peers, id, user_id).await;
                if delivered {
                    send_threads_to_user_id(db, peers, target_id).await;
                }
                if !delivered {
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::System {
                            message: "User is offline.".to_string(),
                        },
                    );
                }
            } else {
                send_to_peer(
                    peers,
                    id,
                    ServerToClient::System {
                        message: "User not found.".to_string(),
                    },
                );
            }
        }
        ClientToServer::FetchThreads => {
            let (_, user_id) = match get_peer_identity(peers, id) {
                Some(info) => info,
                None => {
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::AuthError {
                            message: "Please log in first.".to_string(),
                        },
                    );
                    return;
                }
            };
            send_threads_to_peer(db, peers, id, user_id).await;
        }
        ClientToServer::FetchHistory { target } => {
            let (_, user_id) = match get_peer_identity(peers, id) {
                Some(info) => info,
                None => {
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::AuthError {
                            message: "Please log in first.".to_string(),
                        },
                    );
                    return;
                }
            };
            match target {
                HistoryTarget::Lobby => {
                    if let Ok(messages) = fetch_lobby_history(db, 50).await {
                        send_to_peer(
                            peers,
                            id,
                            ServerToClient::History {
                                target: HistoryTarget::Lobby,
                                messages,
                            },
                        );
                    }
                }
                HistoryTarget::Direct { username } => {
                    if let Ok(Some(target_id)) = get_user_id_by_name(db, &username).await {
                        if let Ok(messages) = fetch_dm_history(db, user_id, target_id, 50).await {
                            send_to_peer(
                                peers,
                                id,
                                ServerToClient::History {
                                    target: HistoryTarget::Direct { username },
                                    messages,
                                },
                            );
                        }
                    } else {
                        send_to_peer(
                            peers,
                            id,
                            ServerToClient::System {
                                message: "User not found.".to_string(),
                            },
                        );
                    }
                }
            }
        }
        ClientToServer::Search { target, query } => {
            let (_, user_id) = match get_peer_identity(peers, id) {
                Some(info) => info,
                None => {
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::AuthError {
                            message: "Please log in first.".to_string(),
                        },
                    );
                    return;
                }
            };
            let trimmed = query.trim();
            if trimmed.is_empty() {
                return;
            }
            match target {
                HistoryTarget::Lobby => {
                    if let Ok(messages) = search_lobby(db, trimmed, 50).await {
                        send_to_peer(
                            peers,
                            id,
                            ServerToClient::SearchResults {
                                target: HistoryTarget::Lobby,
                                query: trimmed.to_string(),
                                messages,
                            },
                        );
                    }
                }
                HistoryTarget::Direct { username } => {
                    if let Ok(Some(target_id)) = get_user_id_by_name(db, &username).await {
                        if let Ok(messages) = search_dm(db, user_id, target_id, trimmed, 50).await {
                            send_to_peer(
                                peers,
                                id,
                                ServerToClient::SearchResults {
                                    target: HistoryTarget::Direct { username },
                                    query: trimmed.to_string(),
                                    messages,
                                },
                            );
                        }
                    }
                }
            }
        }
        ClientToServer::Block { username } => {
            if let Some((_, user_id)) = get_peer_identity(peers, id) {
                if let Ok(Some(target_id)) = get_user_id_by_name(db, &username).await {
                    let _ = block_user(db, user_id, target_id).await;
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::System {
                            message: format!("Blocked {username}."),
                        },
                    );
                }
            }
        }
        ClientToServer::Unblock { username } => {
            if let Some((_, user_id)) = get_peer_identity(peers, id) {
                if let Ok(Some(target_id)) = get_user_id_by_name(db, &username).await {
                    let _ = unblock_user(db, user_id, target_id).await;
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::System {
                            message: format!("Unblocked {username}."),
                        },
                    );
                }
            }
        }
        ClientToServer::Mute { username } => {
            if let Some((_, user_id)) = get_peer_identity(peers, id) {
                if let Ok(Some(target_id)) = get_user_id_by_name(db, &username).await {
                    let _ = mute_user(db, user_id, target_id).await;
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::System {
                            message: format!("Muted {username}."),
                        },
                    );
                }
            }
        }
        ClientToServer::Unmute { username } => {
            if let Some((_, user_id)) = get_peer_identity(peers, id) {
                if let Ok(Some(target_id)) = get_user_id_by_name(db, &username).await {
                    let _ = unmute_user(db, user_id, target_id).await;
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::System {
                            message: format!("Unmuted {username}."),
                        },
                    );
                }
            }
        }
        ClientToServer::Report { username, reason } => {
            if let Some((_, user_id)) = get_peer_identity(peers, id) {
                if let Ok(Some(target_id)) = get_user_id_by_name(db, &username).await {
                    let _ = report_user(db, user_id, target_id, &reason).await;
                    send_to_peer(
                        peers,
                        id,
                        ServerToClient::System {
                            message: "Report submitted.".to_string(),
                        },
                    );
                }
            }
        }
        
        ClientToServer::AddFriend { username } => {
            if let Some((_, user_id)) = get_peer_identity(peers, id) {
                match get_user_id_by_name(db, &username).await {
                    Ok(Some(target_id)) => {
                        match add_friend(db, user_id, target_id).await {
                            Ok(true) => {
                                send_to_peer(peers, id, ServerToClient::AddFriendResult {
                                    _username: username.clone(),
                                    success: true,
                                    message: format!("Friend request sent to {username} (auto-accepted)."),
                                });
                            }
                            Ok(false) => {
                                send_to_peer(peers, id, ServerToClient::AddFriendResult {
                                    _username: username.clone(),
                                    success: false,
                                    message: format!("You are already friends with {username} or request already sent."),
                                });
                            }
                            Err(e) => {
                                send_to_peer(peers, id, ServerToClient::AddFriendResult {
                                    _username: username.clone(),
                                    success: false,
                                    message: format!("Failed to add friend: {e}"),
                                });
                            }
                        }
                    }
                    Ok(None) => {
                        send_to_peer(peers, id, ServerToClient::AddFriendResult {
                            _username: username.clone(),
                            success: false,
                            message: "User not found.".to_string(),
                        });
                    }
                    Err(e) => {
                        send_to_peer(peers, id, ServerToClient::AddFriendResult {
                            _username: username.clone(),
                            success: false,
                            message: format!("Failed to add friend: {e}"),
                        });
                    }
                }
            }
        }
    }
}

fn set_peer_auth(peers: &Arc<Mutex<HashMap<usize, Peer>>>, id: usize, user_id: i64, username: String) {
    if let Ok(mut guard) = peers.lock() {
        if let Some(peer) = guard.get_mut(&id) {
            peer.user_id = Some(user_id);
            peer.username = username;
        }
    }
}

fn is_authed(peers: &Arc<Mutex<HashMap<usize, Peer>>>, id: usize) -> bool {
    peers
        .lock()
        .ok()
        .and_then(|guard| guard.get(&id).and_then(|peer| peer.user_id))
        .is_some()
}

fn get_peer_identity(peers: &Arc<Mutex<HashMap<usize, Peer>>>, id: usize) -> Option<(String, i64)> {
    peers.lock().ok().and_then(|guard| {
        guard.get(&id).and_then(|peer| {
            peer
                .user_id
                .map(|user_id| (peer.username.clone(), user_id))
        })
    })
}

fn allow_rate(rate_limits: &Arc<Mutex<HashMap<i64, RateState>>>, user_id: i64) -> bool {
    let mut guard = match rate_limits.lock() {
        Ok(guard) => guard,
        Err(_) => return true,
    };
    let now = Instant::now();
    let entry = guard.entry(user_id).or_insert(RateState {
        window_start: now,
        count: 0,
    });

    if now.duration_since(entry.window_start) > Duration::from_secs(5) {
        entry.window_start = now;
        entry.count = 0;
    }

    if entry.count >= 5 {
        return false;
    }

    entry.count += 1;
    true
}

async fn send_chat_to_all(
    db: &PgPool,
    peers: &Arc<Mutex<HashMap<usize, Peer>>>,
    sender_id: i64,
    from: &str,
    body: &str,
) {
    let targets = {
        if let Ok(guard) = peers.lock() {
            guard
                .values()
                .filter_map(|peer| peer.user_id.map(|user_id| (user_id, peer.tx.clone())))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    };

    for (target_id, tx) in targets {
        if target_id == sender_id {
            let _ = tx.send(Message::Text(
                serde_json::to_string(&ServerToClient::Chat {
                    from: from.to_string(),
                    body: body.to_string(),
                })
                .unwrap_or_else(|_| "{}".to_string()),
            ));
            continue;
        }
        if is_blocked_or_muted(db, target_id, sender_id).await {
            continue;
        }
        let _ = tx.send(Message::Text(
            serde_json::to_string(&ServerToClient::Chat {
                from: from.to_string(),
                body: body.to_string(),
            })
            .unwrap_or_else(|_| "{}".to_string()),
        ));
    }
}

fn send_dm_to_user(
    peers: &Arc<Mutex<HashMap<usize, Peer>>>,
    target_id: i64,
    from: &str,
    body: &str,
) -> bool {
    let target = {
        if let Ok(guard) = peers.lock() {
            guard
                .values()
                .find(|peer| peer.user_id == Some(target_id))
                .map(|peer| peer.tx.clone())
        } else {
            None
        }
    };

    if let Some(tx) = target {
        let _ = tx.send(Message::Text(
            serde_json::to_string(&ServerToClient::DirectMessage {
                from: from.to_string(),
                body: body.to_string(),
            })
            .unwrap_or_else(|_| "{}".to_string()),
        ));
        true
    } else {
        false
    }
}

fn broadcast_presence(peers: &Arc<Mutex<HashMap<usize, Peer>>>) {
    let users = {
        if let Ok(guard) = peers.lock() {
            guard
                .values()
                .filter(|peer| peer.user_id.is_some())
                .map(|peer| UserStatus {
                    username: peer.username.clone(),
                    away: peer.away.clone(),
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    };

    send_to_all(peers, ServerToClient::Presence { users });
}

fn send_to_peer(peers: &Arc<Mutex<HashMap<usize, Peer>>>, id: usize, payload: ServerToClient) {
    let text = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    let target = {
        if let Ok(guard) = peers.lock() {
            guard.get(&id).map(|peer| peer.tx.clone())
        } else {
            None
        }
    };
    if let Some(tx) = target {
        let _ = tx.send(Message::Text(text));
    }
}

fn send_to_all(peers: &Arc<Mutex<HashMap<usize, Peer>>>, payload: ServerToClient) {
    let text = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    let targets = {
        if let Ok(guard) = peers.lock() {
            guard.values().map(|peer| peer.tx.clone()).collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    };

    for tx in targets {
        let _ = tx.send(Message::Text(text.clone()));
    }
}

async fn send_threads_to_peer(
    db: &PgPool,
    peers: &Arc<Mutex<HashMap<usize, Peer>>>,
    id: usize,
    user_id: i64,
) {
    if let Ok(users) = fetch_recent_threads(db, user_id, 10).await {
        send_to_peer(peers, id, ServerToClient::Threads { users });
    }
}

async fn send_threads_to_user_id(
    db: &PgPool,
    peers: &Arc<Mutex<HashMap<usize, Peer>>>,
    user_id: i64,
) {
    let target = {
        if let Ok(guard) = peers.lock() {
            guard
                .iter()
                .find_map(|(id, peer)| {
                    if peer.user_id == Some(user_id) {
                        Some(*id)
                    } else {
                        None
                    }
                })
        } else {
            None
        }
    };
    if let Some(id) = target {
        send_threads_to_peer(db, peers, id, user_id).await;
    }
}

async fn init_db(db: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS friends (\
                user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
                friend_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
                UNIQUE(user_id, friend_user_id)\
            )"
        )
        .execute(db)
        .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (\
            id SERIAL PRIMARY KEY,\
            username TEXT NOT NULL UNIQUE,\
            password_hash TEXT NOT NULL,\
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
        )",
    )
    .execute(db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS blocks (\
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
            blocked_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
            UNIQUE(user_id, blocked_user_id)\
        )",
    )
    .execute(db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS mutes (\
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
            muted_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
            UNIQUE(user_id, muted_user_id)\
        )",
    )
    .execute(db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS reports (\
            id SERIAL PRIMARY KEY,\
            reporter_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
            reported_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
            reason TEXT NOT NULL,\
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
        )",
    )
    .execute(db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (\
            id SERIAL PRIMARY KEY,\
            sender_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
            recipient_id INTEGER REFERENCES users(id) ON DELETE CASCADE,\
            body TEXT NOT NULL,\
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
        )",
    )
    .execute(db)
    .await?;

    sqlx::query(
           "CREATE INDEX IF NOT EXISTS messages_body_fts \
            ON messages USING GIN (to_tsvector('english', body))",
    )
    .execute(db)
    .await?;

    Ok(())
}

async fn insert_message(
    db: &PgPool,
    sender_id: i64,
    recipient_id: Option<i64>,
    body: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO messages (sender_id, recipient_id, body) VALUES ($1, $2, $3)")
        .bind(sender_id)
        .bind(recipient_id)
        .bind(body)
        .execute(db)
        .await?;
    Ok(())
}

async fn fetch_lobby_history(db: &PgPool, limit: i64) -> Result<Vec<MessageRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT u.username, m.body, m.created_at\
         FROM messages m\
         JOIN users u ON u.id = m.sender_id\
         WHERE m.recipient_id IS NULL\
         ORDER BY m.created_at DESC\
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(db)
    .await?;

    let mut messages = rows
        .into_iter()
        .map(|row| MessageRecord {
            from: row.get::<String, _>("username"),
            body: row.get::<String, _>("body"),
            at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect::<Vec<_>>();
    messages.reverse();
    Ok(messages)
}

async fn fetch_dm_history(
    db: &PgPool,
    user_id: i64,
    peer_id: i64,
    limit: i64,
) -> Result<Vec<MessageRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT u.username, m.body, m.created_at\
         FROM messages m\
         JOIN users u ON u.id = m.sender_id\
         WHERE (m.sender_id = $1 AND m.recipient_id = $2)\
            OR (m.sender_id = $2 AND m.recipient_id = $1)\
         ORDER BY m.created_at DESC\
         LIMIT $3",
    )
    .bind(user_id)
    .bind(peer_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let mut messages = rows
        .into_iter()
        .map(|row| MessageRecord {
            from: row.get::<String, _>("username"),
            body: row.get::<String, _>("body"),
            at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect::<Vec<_>>();
    messages.reverse();
    Ok(messages)
}

async fn fetch_recent_threads(
    db: &PgPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query(
        "WITH latest AS (\
            SELECT CASE\
                WHEN sender_id = $1 THEN recipient_id\
                ELSE sender_id\
            END AS other_id, MAX(created_at) AS last_at\
            FROM messages\
            WHERE (sender_id = $1 OR recipient_id = $1)\
              AND recipient_id IS NOT NULL\
            GROUP BY other_id\
        )\
        SELECT u.username\
        FROM latest\
        JOIN users u ON u.id = latest.other_id\
        ORDER BY latest.last_at DESC\
        LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| row.get::<String, _>("username"))
        .collect())
}

async fn search_lobby(
    db: &PgPool,
    query: &str,
    limit: i64,
) -> Result<Vec<MessageRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT u.username, m.body, m.created_at\
         FROM messages m\
         JOIN users u ON u.id = m.sender_id\
         WHERE m.recipient_id IS NULL\
           AND to_tsvector('english', m.body) @@ plainto_tsquery('english', $1)\
         ORDER BY m.created_at DESC\
         LIMIT $2",
    )
    .bind(query)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let mut messages = rows
        .into_iter()
        .map(|row| MessageRecord {
            from: row.get::<String, _>("username"),
            body: row.get::<String, _>("body"),
            at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect::<Vec<_>>();
    messages.reverse();
    Ok(messages)
}

async fn search_dm(
    db: &PgPool,
    user_id: i64,
    peer_id: i64,
    query: &str,
    limit: i64,
) -> Result<Vec<MessageRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT u.username, m.body, m.created_at\
         FROM messages m\
         JOIN users u ON u.id = m.sender_id\
         WHERE ((m.sender_id = $1 AND m.recipient_id = $2)\
            OR (m.sender_id = $2 AND m.recipient_id = $1))\
           AND to_tsvector('english', m.body) @@ plainto_tsquery('english', $3)\
         ORDER BY m.created_at DESC\
         LIMIT $4",
    )
    .bind(user_id)
    .bind(peer_id)
    .bind(query)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let mut messages = rows
        .into_iter()
        .map(|row| MessageRecord {
            from: row.get::<String, _>("username"),
            body: row.get::<String, _>("body"),
            at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect::<Vec<_>>();
    messages.reverse();
    Ok(messages)
}

async fn create_user(db: &PgPool, username: &str, password: &str) -> Result<i64, String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| "failed to hash password")?
        .to_string();

    let row = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id::bigint AS id")
        .bind(username)
        .bind(hash)
        .fetch_one(db)
        .await
        .map_err(|err| {
            if err.to_string().contains("unique") {
                "Username already exists".to_string()
            } else {
                "Failed to create account".to_string()
            }
        })?;
    Ok(row.get::<i64, _>("id"))
}

async fn verify_user(db: &PgPool, username: &str, password: &str) -> Result<i64, String> {
    let row = sqlx::query("SELECT id::bigint AS id, password_hash FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(db)
        .await
        .map_err(|_| "Login failed".to_string())?;

    let row = match row {
        Some(row) => row,
        None => return Err("Invalid username or password".to_string()),
    };

    let hash: String = row.get("password_hash");
    let parsed_hash = PasswordHash::new(&hash).map_err(|_| "Login failed".to_string())?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| "Invalid username or password".to_string())?;

    Ok(row.get::<i64, _>("id"))
}

async fn get_user_id_by_name(db: &PgPool, username: &str) -> Result<Option<i64>, sqlx::Error> {
    let row = sqlx::query("SELECT id::bigint AS id FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(db)
        .await?;
    Ok(row.map(|row| row.get::<i64, _>("id")))
}

async fn block_user(db: &PgPool, user_id: i64, target_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO blocks (user_id, blocked_user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .bind(target_id)
        .execute(db)
        .await?;
    Ok(())
}

async fn unblock_user(db: &PgPool, user_id: i64, target_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blocks WHERE user_id = $1 AND blocked_user_id = $2")
        .bind(user_id)
        .bind(target_id)
        .execute(db)
        .await?;
    Ok(())
}

async fn mute_user(db: &PgPool, user_id: i64, target_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO mutes (user_id, muted_user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .bind(target_id)
        .execute(db)
        .await?;
    Ok(())
}

async fn unmute_user(db: &PgPool, user_id: i64, target_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mutes WHERE user_id = $1 AND muted_user_id = $2")
        .bind(user_id)
        .bind(target_id)
        .execute(db)
        .await?;
    Ok(())
}

async fn report_user(db: &PgPool, user_id: i64, target_id: i64, reason: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO reports (reporter_user_id, reported_user_id, reason) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(target_id)
        .bind(reason)
        .execute(db)
        .await?;
    Ok(())
}

// Add a friend (auto-accept, bidirectional)
async fn add_friend(db: &PgPool, user_id: i64, target_id: i64) -> Result<bool, sqlx::Error> {
    if user_id == target_id {
        return Ok(false);
    }
    // Check if already friends
    let already = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM friends WHERE user_id = $1 AND friend_user_id = $2"
    )
    .bind(user_id)
    .bind(target_id)
    .fetch_optional(db)
    .await?
    .is_some();
    if already {
        return Ok(false);
    }
    // Insert both directions
    sqlx::query("INSERT INTO friends (user_id, friend_user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .bind(target_id)
        .execute(db)
        .await?;
    sqlx::query("INSERT INTO friends (user_id, friend_user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(target_id)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(true)
}

async fn is_blocked_or_muted(db: &PgPool, recipient_id: i64, sender_id: i64) -> bool {
    let blocked = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM blocks WHERE user_id = $1 AND blocked_user_id = $2",
    )
    .bind(recipient_id)
    .bind(sender_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .is_some();

    if blocked {
        return true;
    }

    sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM mutes WHERE user_id = $1 AND muted_user_id = $2",
    )
    .bind(recipient_id)
    .bind(sender_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .is_some()
}

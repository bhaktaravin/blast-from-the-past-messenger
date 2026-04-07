use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;

use chrono::Utc;
use eframe::egui;

// E2E encryption (native only — x25519 + chacha20poly1305)
#[cfg(not(target_arch = "wasm32"))]
use x25519_dalek::{PublicKey, StaticSecret};
#[cfg(not(target_arch = "wasm32"))]
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
#[cfg(not(target_arch = "wasm32"))]
use chacha20poly1305::aead::{Aead, KeyInit};
#[cfg(not(target_arch = "wasm32"))]
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

// Platform-specific timing
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
type Instant = u32;  // Frame counter on web

// Native-only imports
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use futures_util::{Sink, SinkExt, StreamExt};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::connect_async;
#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::tungstenite::Message;

// Web-only imports
#[cfg(target_arch = "wasm32")]
use web_sys::WebSocket;

// Web stub for mpsc (use a simple Arc<Mutex<Vec>> based queue)
#[cfg(target_arch = "wasm32")]
mod mpsc {
    use std::sync::{Arc, Mutex};

    pub struct UnboundedSender<T> {
        queue: Arc<Mutex<Vec<T>>>,
    }

    impl<T> Clone for UnboundedSender<T> {
        fn clone(&self) -> Self {
            Self { queue: self.queue.clone() }
        }
    }

    impl<T> UnboundedSender<T> {
        pub fn send(&self, msg: T) -> Result<(), ()> {
            self.queue.lock().map(|mut q| q.push(msg)).map_err(|_| ())
        }
    }

    pub struct UnboundedReceiver<T> {
        queue: Arc<Mutex<Vec<T>>>,
    }

    impl<T> UnboundedReceiver<T> {
        pub fn try_recv(&mut self) -> Result<T, ()> {
            self.queue
                .lock()
                .map_err(|_| ())?
                .first()
                .ok_or(())
                .and_then(|_| self.queue.lock().map_err(|_| ()).map(|mut q| q.remove(0)))
        }

        pub async fn recv(&mut self) -> Option<T> {
            loop {
                if let Ok(msg) = self.try_recv() {
                    return Some(msg);
                }
                gloo_timers::future::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }

    pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
        let queue = Arc::new(Mutex::new(Vec::new()));
        (
            UnboundedSender { queue: queue.clone() },
            UnboundedReceiver { queue },
        )
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "client"))]
use chatmessagediscordclone::audio::{AudioManager, SoundEffect};
use chatmessagediscordclone::protocol::{
    ClientToServer, HistoryTarget, ServerToClient, UserStatus,
};

// Stub audio for web
#[cfg(target_arch = "wasm32")]
mod audio_stub {
    #[derive(Debug, Clone, Copy)]
    pub enum SoundEffect {
        BuddySignOn,
        BuddySignOff,
        MessageReceived,
        MessageSent,
    }

    pub struct AudioManager {
        enabled: bool,
        volume: f32,
    }

    impl AudioManager {
        pub fn new() -> Self {
            Self {
                enabled: false,
                volume: 0.8,
            }
        }

        pub fn play(&self, _effect: SoundEffect) {
            // No-op on web
        }

        pub fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }

        pub fn set_volume(&mut self, volume: f32) {
            self.volume = volume.clamp(0.0, 1.0);
        }
    }
}

#[cfg(target_arch = "wasm32")]
use audio_stub::{AudioManager, SoundEffect};

#[derive(Debug, Clone)]
struct ChatMessage {
    from: String,
    body: String,
    at: String,
    id: Option<i64>,
    read_count: Option<i32>,
}

struct Toast {
    text: String,
    kind: ToastKind,
    ttl: f32,
}

enum ToastKind {
    Info,
    Success,
    Error,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    SignIn,
    Chat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthMode {
    Login,
    Register,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ChatTarget {
    Lobby,
    Direct(String),
    Room(String),
}

enum UiToNet {
    Connect {
        url: String,
        username: String,
        password: String,
        mode: AuthMode,
    },
    SendChat { body: String },
    SendDirect { to: String, body: String },
    /// Send our X25519 public key to a peer to initiate E2E
    ExchangeKey { to: String, public_key: String },
    /// Send an E2E encrypted DM (body is base64 nonce+ciphertext)
    SendEncryptedDirect { to: String, encrypted_body: String },
    FetchHistory { target: ChatTarget },
    FetchThreads,
    Search { target: ChatTarget, query: String },
    SetAway { away: Option<String> },
    Block { username: String },
    Unblock { username: String },
    Mute { username: String },
    Unmute { username: String },
    Report { username: String, reason: String },
    Disconnect,
    AddFriend { username: String },
    AcceptFriendRequest { username: String },
    DeclineFriendRequest { username: String },
    CreateChatRoom { name: String },
    JoinChatRoom { room_id: String },
    LeaveChatRoom { room_id: String },
    SendRoomMessage { room_id: String, body: String },
    FetchChatRooms,
    FetchRoomMembers { room_id: String },
    StartTyping { room_id: String },
    StopTyping { room_id: String },
    MarkMessageAsRead { message_id: i64 },
    EditMessage { message_id: i64, new_body: String },
    DeleteMessage { message_id: i64 },
    ReactToMessage { message_id: i64, emoji: String },
    Nudge { to: String },
    Wink { to: String, emoji: String },
    SetStatus { status: Option<String> },
    SetBio { bio: String },
    FetchProfile { username: String },
    ReplyToMessage { reply_to_id: i64, body: String },
    ReplyToDirect { to: String, reply_to_id: i64, body: String },
    SetAvatar { avatar_data: String },
}

enum NetToUi {
    Connected,
    Disconnected,
    Presence(Vec<UserStatus>),
    Chat { from: String, body: String },
    DirectMessage { from: String, body: String },
    /// Peer sent us their public key for E2E key exchange
    KeyExchange { from: String, public_key: String },
    /// Received an E2E encrypted DM
    EncryptedDirectMessage { from: String, encrypted_body: String },
    Threads(Vec<String>),
    History { target: ChatTarget, messages: Vec<ChatMessage> },
    SearchResults {
        target: ChatTarget,
        query: String,
        messages: Vec<ChatMessage>,
    },
    AuthOk { username: String },
    AuthError(String),
    System(String),
    Error(String),
    AddFriendResult { _username: String, success: bool, message: String },
    FriendRequest { from: String },
    FriendRequestResult { username: String, accepted: bool },
    ChatRoomCreated { room_id: String, name: String },
    ChatRoomList { rooms: Vec<(String, String, i32)> }, // (id, name, member_count)
    RoomMessage { room_id: String, from: String, body: String, message_id: i64 },
    UserJoinedRoom { room_id: String, username: String },
    UserLeftRoom { room_id: String, username: String },
    RoomMembers { room_id: String, members: Vec<String> },
    UserTyping { room_id: String, username: String },
    UserStoppedTyping { room_id: String, username: String },
    ReadReceipt { message_id: i64, read_by: String },
    MessageEdited { message_id: i64, new_body: String },
    MessageDeleted { message_id: i64 },
    MessageReaction { message_id: i64, emoji: String, from: String },
    Nudged { from: String },
    Winked { from: String, emoji: String },
    ProfileData { username: String, bio: String, status: Option<String>, joined: String, avatar_url: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Theme {
    Light,
    Dark,
    MidnightAmber,
}

struct NetworkHandle {
    tx: mpsc::UnboundedSender<UiToNet>,
    rx: std_mpsc::Receiver<NetToUi>,
}

struct AolApp {
    screen: Screen,
    network: NetworkHandle,
    connected: bool,
    status: String,
    theme: Theme,
    startup_repaint_left: u32,
    show_background: bool,
    auth_mode: AuthMode,
    logged_in_user: Option<String>,
    server_url: String,
    username: String,
    password: String,
    confirm_password: String,
    show_password: bool,
    show_confirm_password: bool,
    remember_me: bool,
    away_text: String,
    chat_input: String,
    dm_target: String,
    selected_target: ChatTarget,
    report_reason: String,
    logging_in: bool,
    login_started_at: Option<Instant>,
    login_frame_count: u32,  // Web: tracks elapsed frames for 3-second timeout
    login_error: bool,  // Track if login failed for red flash effect
    login_error_time: f32,  // Time since error for fade effect
    login_success: bool,  // Track if login succeeded for green flash effect
    login_success_time: f32,  // Time since success for transition
    search_query: String,
    search_in_progress: bool,
    search_results: HashMap<ChatTarget, SearchResult>,
    buddies: Vec<UserStatus>,
    recent_threads: Vec<String>,
    messages: std::collections::HashMap<ChatTarget, Vec<ChatMessage>>,
    toast: Option<Toast>,
    // Add Friend modal state
    show_add_friend_modal: bool,
    add_friend_name: String,
    // Incoming friend requests
    pending_friend_requests: Vec<String>,
    show_friend_requests_modal: bool,
    // Audio settings
    audio_manager: AudioManager,
    sound_enabled: bool,
    sound_volume: f32,
    // Chat rooms
    chat_rooms: Vec<(String, String, i32)>, // (id, name, member_count)
    show_room_creation_modal: bool,
    new_room_name: String,
    // Typing indicators
    typing_users: std::collections::HashMap<String, Vec<String>>, // room_id -> list of users typing
    typing_timeout: std::collections::HashMap<String, std::time::Instant>, // user -> when they stop typing
    // Read receipts
    message_read_status: std::collections::HashMap<i64, Vec<String>>,
    // Unread message counts per chat target
    unread_counts: HashMap<ChatTarget, usize>,
    // Message reactions: message_id -> list of (emoji, username)
    reactions: HashMap<i64, Vec<(String, String)>>,
    // Nudge animation: time remaining for screen shake
    nudge_time: f32,
    nudge_from: Option<String>,
    // Wink animation: emoji, position, time, from
    wink_animation: Option<(String, f32, f32, String)>, // (emoji, x_pos, time, from) // message_id -> list of users who read
    // E2E encryption: shared secrets keyed by peer username (native only)
    // Stored as raw 32-byte arrays to avoid lifetime issues with x25519_dalek types
    #[cfg(not(target_arch = "wasm32"))]
    e2e_shared_secrets: HashMap<String, [u8; 32]>,
    // Pending outbound secret (waiting for peer's public key ack)
    #[cfg(not(target_arch = "wasm32"))]
    e2e_pending_secret: Option<(String, [u8; 32])>,
    // Login screen animation state
    login_anim_time: f32,
    login_typewriter_pos: usize,
    login_scanline_offset: f32,
    // Message editing state
    editing_message: Option<(i64, String)>, // (message_id, current edit text)
    // Saved credentials path
    credentials_path: std::path::PathBuf,
    // Reply state: (message_id, from, body_snippet)
    replying_to: Option<(i64, String, String)>,
    // Custom status input
    custom_status: String,
    // Profile modal
    viewing_profile: Option<String>,
    profile_cache: HashMap<String, (String, Option<String>, String, Option<String>)>, // (bio, status, joined, avatar_url)
    bio_edit: String,
    bio_editing: bool,
    // Avatar upload
    show_avatar_modal: bool,
    avatar_upload_path: String,
    // Buddy groups
    buddy_groups: HashMap<String, Vec<String>>, // group_name -> list of usernames
    show_group_modal: bool,
    group_modal_username: Option<String>,
    new_group_name: String,
    // Boot sequence
    boot_done: bool,
    boot_line: usize,
    boot_line_timer: f32,
    // Matrix rain
    matrix_cols: Vec<MatrixCol>,
    matrix_initialized: bool,
    // Dial-up modem animation
    modem_line: usize,
    modem_line_timer: f32,
    modem_char_pos: usize,
}

struct SearchResult {
    query: String,
    messages: Vec<ChatMessage>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SavedCredentials {
    username: String,
    password: String,
    server_url: String,
}

/// One column of falling characters in the matrix rain background
struct MatrixCol {
    x: f32,
    y: f32,
    speed: f32,
    chars: Vec<char>,
    head: usize,
    length: usize,
}

impl AolApp {
    /// Sanitize and validate user input to prevent injection and external manipulation
    fn sanitize_input(input: &str) -> String {
        input
            .trim()
            .replace('\0', "")  // Remove null bytes
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())  // Remove control chars except whitespace
            .take(1024)  // Limit length
            .collect()
    }
    
    /// Sanitize username/email input
    fn sanitize_username(input: &str) -> String {
        input
            .trim()
            .replace('\0', "")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')  // Only safe chars
            .take(32)  // Limit length
            .collect()
    }
    
    fn send_add_friend(&mut self) {
        let name = Self::sanitize_username(&self.add_friend_name);
        if !name.is_empty() {
            let _ = self.network.tx.send(UiToNet::AddFriend { username: name.clone() });
            self.add_friend_name.clear();
            self.show_toast(format!("Added {0}", name), ToastKind::Success);
        } else {
            self.show_toast("Please enter a screen name.".to_string(), ToastKind::Error);
        }
    }
    
    fn accept_friend_request(&mut self, username: String) {
        let _ = self.network.tx.send(UiToNet::AcceptFriendRequest { username: username.clone() });
    }
    
    fn decline_friend_request(&mut self, username: String) {
        let _ = self.network.tx.send(UiToNet::DeclineFriendRequest { username: username.clone() });
    }
    
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        let mono = egui::FontId::new(16.0, egui::FontFamily::Monospace);
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Monospace));
        style.text_styles.insert(egui::TextStyle::Body, mono.clone());
        style
            .text_styles
            .insert(egui::TextStyle::Button, mono.clone());
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::new(12.0, egui::FontFamily::Monospace));
        cc.egui_ctx.set_style(style);
        apply_theme(&cc.egui_ctx, Theme::MidnightAmber);

        Self {
            screen: Screen::SignIn,
            network: spawn_network(),
            connected: false,
            status: "Offline".to_string(),
            theme: Theme::MidnightAmber,
            startup_repaint_left: 120,
            show_background: false,
            auth_mode: AuthMode::Login,
            logged_in_user: None,
            server_url: "wss://blast-from-the-past-messenger-production.up.railway.app".to_string(),
            username: "RetroUser".to_string(),
            password: String::new(),
            confirm_password: String::new(),
            show_password: false,
            show_confirm_password: false,
            remember_me: true,  // Default to true for convenience
            away_text: String::new(),
            chat_input: String::new(),
            dm_target: String::new(),
            selected_target: ChatTarget::Lobby,
            report_reason: String::new(),
            logging_in: false,
            login_started_at: None,
            login_frame_count: 0,
            login_error: false,
            login_error_time: 0.0,
            login_success: false,
            login_success_time: 0.0,
            search_query: String::new(),
            search_in_progress: false,
            search_results: HashMap::new(),
            buddies: Vec::new(),
            recent_threads: Vec::new(),
            messages: HashMap::new(),
            toast: None,
            show_add_friend_modal: false,
            add_friend_name: String::new(),
            audio_manager: AudioManager::new(),
            sound_enabled: true,
            sound_volume: 0.8,
            pending_friend_requests: Vec::new(),
            show_friend_requests_modal: false,
            chat_rooms: Vec::new(),
            show_room_creation_modal: false,
            new_room_name: String::new(),
            typing_users: HashMap::new(),
            typing_timeout: HashMap::new(),
            message_read_status: HashMap::new(),
            unread_counts: HashMap::new(),
            reactions: HashMap::new(),
            nudge_time: 0.0,
            nudge_from: None,
            wink_animation: None,
            #[cfg(not(target_arch = "wasm32"))]
            e2e_shared_secrets: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            e2e_pending_secret: None,
            login_anim_time: 0.0,
            login_typewriter_pos: 0,
            login_scanline_offset: 0.0,
            boot_done: false,
            boot_line: 0,
            boot_line_timer: 0.0,
            matrix_cols: Vec::new(),
            matrix_initialized: false,
            modem_line: 0,
            modem_line_timer: 0.0,
            modem_char_pos: 0,
            editing_message: None,
            credentials_path: {
                let mut p = dirs::config_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                p.push("blast-from-the-past");
                p
            },
            replying_to: None,
            custom_status: String::new(),
            viewing_profile: None,
            profile_cache: HashMap::new(),
            bio_edit: String::new(),
            bio_editing: false,
            show_avatar_modal: false,
            avatar_upload_path: String::new(),
            buddy_groups: {
                let mut groups = HashMap::new();
                groups.insert("Friends".to_string(), Vec::new());
                groups.insert("Work".to_string(), Vec::new());
                groups.insert("Family".to_string(), Vec::new());
                groups
            },
            show_group_modal: false,
            group_modal_username: None,
            new_group_name: String::new(),
        }
    }

    fn credentials_file(&self) -> std::path::PathBuf {
        self.credentials_path.join("credentials.json")
    }

    fn save_credentials(&self) {
        if !self.remember_me {
            // If remember me is unchecked, delete saved credentials
            let _ = std::fs::remove_file(self.credentials_file());
            return;
        }
        
        let creds = SavedCredentials {
            username: self.username.clone(),
            password: self.password.clone(),
            server_url: self.server_url.clone(),
        };
        if let Ok(json) = serde_json::to_string(&creds) {
            let _ = std::fs::create_dir_all(&self.credentials_path);
            let _ = std::fs::write(self.credentials_file(), json);
        }
    }

    fn load_credentials(&mut self) {
        if let Ok(data) = std::fs::read_to_string(self.credentials_file()) {
            if let Ok(creds) = serde_json::from_str::<SavedCredentials>(&data) {
                if !creds.username.is_empty() { 
                    self.username = creds.username;
                    self.remember_me = true;
                }
                if !creds.password.is_empty() { 
                    self.password = creds.password;
                }
                // Don't load server_url from saved credentials - always use default
                // This prevents issues when switching servers
            }
        }
        // Load buddy groups
        self.load_buddy_groups();
    }

    fn buddy_groups_file(&self) -> std::path::PathBuf {
        self.credentials_path.join("buddy_groups.json")
    }

    fn save_buddy_groups(&self) {
        if let Ok(json) = serde_json::to_string(&self.buddy_groups) {
            let _ = std::fs::create_dir_all(&self.credentials_path);
            let _ = std::fs::write(self.buddy_groups_file(), json);
        }
    }

    fn load_buddy_groups(&mut self) {
        if let Ok(data) = std::fs::read_to_string(self.buddy_groups_file()) {
            if let Ok(groups) = serde_json::from_str::<HashMap<String, Vec<String>>>(&data) {
                self.buddy_groups = groups;
            }
        }
    }

    fn draw_background(&self, ctx: &egui::Context) {
        let rect = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("retro_bg"),
        ));
        let steps = 12;
        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let (r, g, b) = match self.theme {
                Theme::Light => (
                    (240.0 + 10.0 * t) as u8,
                    (230.0 + 18.0 * t) as u8,
                    (210.0 + 30.0 * t) as u8,
                ),
                Theme::Dark => (
                    (22.0 + 6.0 * t) as u8,
                    (24.0 + 6.0 * t) as u8,
                    (28.0 + 10.0 * t) as u8,
                ),
                Theme::MidnightAmber => (
                    (24.0 + 8.0 * t) as u8,
                    (22.0 + 6.0 * t) as u8,
                    (18.0 + 4.0 * t) as u8,
                ),
            };
            let color = egui::Color32::from_rgb(r, g, b);
            let y0 = rect.top() + rect.height() * (i as f32 / steps as f32);
            let y1 = rect.top() + rect.height() * ((i + 1) as f32 / steps as f32);
            let band = egui::Rect::from_min_max(
                egui::pos2(rect.left(), y0),
                egui::pos2(rect.right(), y1),
            );
            painter.rect_filled(band, 0.0, color);
        }
    }

    fn process_net_events(&mut self) {
        while let Ok(event) = self.network.rx.try_recv() {
            match event {
                NetToUi::Connected => {
                    self.connected = true;
                    self.status = "Online".to_string();
                }
                NetToUi::Disconnected => {
                    self.connected = false;
                    self.status = "Disconnected".to_string();
                    self.screen = Screen::SignIn;
                    self.logged_in_user = None;
                    self.logging_in = false;
                    self.login_started_at = None;
                }
                NetToUi::Presence(users) => {
                    // Detect buddy sign-ons and sign-offs
                    use std::collections::HashSet;
                    let old_buddies: HashSet<_> =
                        self.buddies.iter().map(|u| u.username.clone()).collect();
                    let new_buddies: HashSet<_> =
                        users.iter().map(|u| u.username.clone()).collect();

                    // Collect usernames that signed on/off (owned Strings)
                    let signed_on: Vec<String> = new_buddies
                        .difference(&old_buddies)
                        .cloned()
                        .collect();
                    let signed_off: Vec<String> = old_buddies
                        .difference(&new_buddies)
                        .cloned()
                        .collect();

                    // Update buddies list
                    self.buddies = users;

                    // Now we can safely use self mutably
                    for username in signed_on {
                        self.show_toast(
                            format!("{} signed on", username),
                            ToastKind::Info
                        );
                        self.audio_manager.play(SoundEffect::BuddySignOn);
                    }

                    for username in signed_off {
                        self.show_toast(
                            format!("{} signed off", username),
                            ToastKind::Info
                        );
                        self.audio_manager.play(SoundEffect::BuddySignOff);
                    }
                }
                NetToUi::Chat { from, body } => {
                    let entry = self.messages.entry(ChatTarget::Lobby).or_default();
                    entry.push(ChatMessage {
                        from,
                        body,
                        at: Utc::now().to_rfc3339(),
                        id: None,
                        read_count: None,
                    });
                    self.audio_manager.play(SoundEffect::MessageReceived);
                    if self.selected_target != ChatTarget::Lobby {
                        *self.unread_counts.entry(ChatTarget::Lobby).or_default() += 1;
                    }
                }
                NetToUi::DirectMessage { from, body } => {
                    let target = ChatTarget::Direct(from.clone());
                    let entry = self.messages.entry(target.clone()).or_default();
                    entry.push(ChatMessage {
                        from: from.clone(),
                        body,
                        at: Utc::now().to_rfc3339(),
                        id: None,
                        read_count: None,
                    });
                    self.audio_manager.play(SoundEffect::MessageReceived);
                    if self.selected_target != target {
                        *self.unread_counts.entry(target).or_default() += 1;
                    }
                    // Desktop notification for DMs
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = notify_rust::Notification::new()
                            .summary(&format!("Message from {}", from))
                            .body("You have a new direct message")
                            .timeout(notify_rust::Timeout::Milliseconds(4000))
                            .show();
                    }
                }
                NetToUi::KeyExchange { from, public_key } => {
                    #[cfg(not(target_arch = "wasm32"))]
                    self.complete_e2e(&from.clone(), &public_key.clone());
                }
                NetToUi::EncryptedDirectMessage { from, encrypted_body } => {
                    #[cfg(not(target_arch = "wasm32"))]
                    let body = self.decrypt_dm(&from, &encrypted_body)
                        .unwrap_or_else(|| "🔒 [encrypted — could not decrypt]".to_string());
                    #[cfg(target_arch = "wasm32")]
                    let body = "🔒 [encrypted]".to_string();
                    let target = ChatTarget::Direct(from.clone());
                    let entry = self.messages.entry(target).or_default();
                    entry.push(ChatMessage {
                        from,
                        body,
                        at: Utc::now().to_rfc3339(),
                        id: None,
                        read_count: None,
                    });
                    self.audio_manager.play(SoundEffect::MessageReceived);
                }                NetToUi::Threads(users) => {
                    self.recent_threads = users;
                }
                NetToUi::History { target, messages } => {
                    self.messages.insert(target, messages);
                }
                NetToUi::SearchResults {
                    target,
                    query,
                    messages,
                } => {
                    self.search_results.insert(
                        target,
                        SearchResult {
                            query,
                            messages,
                        },
                    );
                    self.search_in_progress = false;
                }
                NetToUi::AddFriendResult { _username: _, success, message } => {
                    let kind = if success { ToastKind::Success } else { ToastKind::Error };
                    self.show_toast(message, kind);
                }
                NetToUi::AuthOk { username } => {
                    self.logged_in_user = Some(username);
                    self.status = match self.auth_mode {
                        AuthMode::Register => "Account created. Logged in.".to_string(),
                        AuthMode::Login => "Logged in.".to_string(),
                    };
                    self.show_toast(self.status.clone(), ToastKind::Success);
                    // Don't switch screen immediately - show success animation first
                    self.logging_in = false;
                    self.login_started_at = None;
                    self.login_success = true;
                    self.login_success_time = 0.0;
                    self.save_credentials();
                    let _ = self
                        .network
                        .tx
                        .send(UiToNet::FetchHistory {
                            target: ChatTarget::Lobby,
                        });
                    let _ = self.network.tx.send(UiToNet::FetchThreads);
                }
                NetToUi::AuthError(message) => {
                    self.status = message;
                    self.show_toast(self.status.clone(), ToastKind::Error);
                    self.logging_in = false;
                    self.login_started_at = None;
                    self.login_error = true;
                    self.login_error_time = 0.0;
                }
                NetToUi::System(message) => {
                    let entry = self.messages.entry(ChatTarget::Lobby).or_default();
                    entry.push(ChatMessage {
                        from: "System".to_string(),
                        body: message,
                        at: Utc::now().to_rfc3339(),
                        id: None,
                        read_count: None,
                    });
                    self.show_toast("System message received".to_string(), ToastKind::Info);
                }
                NetToUi::Error(message) => {
                    self.status = format!("Error: {message}");
                    self.show_toast(self.status.clone(), ToastKind::Error);
                    self.logging_in = false;
                    self.login_started_at = None;
                }
                NetToUi::FriendRequest { from } => {
                    if !self.pending_friend_requests.contains(&from) {
                        self.pending_friend_requests.push(from.clone());
                        self.show_toast(
                            format!("{} wants to add you", from),
                            ToastKind::Info,
                        );
                    }
                }
                NetToUi::FriendRequestResult { username, accepted } => {
                    let msg = if accepted {
                        format!("Accepted friend request from {}", username)
                    } else {
                        format!("Declined friend request from {}", username)
                    };
                    self.show_toast(msg, ToastKind::Success);
                    self.pending_friend_requests.retain(|u| u != &username);
                }
                NetToUi::ChatRoomCreated { room_id, name } => {
                    self.show_toast(format!("Created room: {}", name), ToastKind::Success);
                    self.selected_target = ChatTarget::Room(room_id);
                }
                NetToUi::ChatRoomList { rooms } => {
                    self.chat_rooms = rooms;
                }
                NetToUi::RoomMessage { room_id, from, body, message_id } => {
                    let target = ChatTarget::Room(room_id);
                    let entry = self.messages.entry(target).or_default();
                    entry.push(ChatMessage {
                        from,
                        body,
                        at: Utc::now().to_rfc3339(),
                        id: Some(message_id),
                        read_count: Some(0),
                    });
                    self.audio_manager.play(SoundEffect::MessageReceived);
                }
                NetToUi::UserJoinedRoom { room_id, username } => {
                    self.show_toast(format!("{} joined the room", username), ToastKind::Info);
                }
                NetToUi::UserLeftRoom { room_id, username } => {
                    self.show_toast(format!("{} left the room", username), ToastKind::Info);
                }
                NetToUi::RoomMembers { room_id, members } => {
                    // Store member list - could be used for UI display
                }
                NetToUi::UserTyping { room_id, username } => {
                    let key = format!("{}:{}", room_id, username);
                    self.typing_timeout.insert(key, std::time::Instant::now());
                    let users = self.typing_users.entry(room_id).or_default();
                    if !users.contains(&username) {
                        users.push(username);
                    }
                }
                NetToUi::UserStoppedTyping { room_id, username } => {
                    let key = format!("{}:{}", room_id, username);
                    self.typing_timeout.remove(&key);
                    if let Some(users) = self.typing_users.get_mut(&room_id) {
                        users.retain(|u| u != &username);
                    }
                }
                NetToUi::ReadReceipt { message_id, read_by } => {
                    self.message_read_status.entry(message_id).or_default().push(read_by);
                }
                NetToUi::MessageEdited { message_id, new_body } => {
                    for msgs in self.messages.values_mut() {
                        for msg in msgs.iter_mut() {
                            if msg.id == Some(message_id) {
                                msg.body = format!("{} ✏️", new_body);
                                break;
                            }
                        }
                    }
                }
                NetToUi::MessageDeleted { message_id } => {
                    for msgs in self.messages.values_mut() {
                        for msg in msgs.iter_mut() {
                            if msg.id == Some(message_id) {
                                msg.body = "[deleted]".to_string();
                                break;
                            }
                        }
                    }
                }
                NetToUi::MessageReaction { message_id, emoji, from } => {
                    let list = self.reactions.entry(message_id).or_default();
                    // Toggle: remove if same user+emoji already exists, else add
                    if let Some(pos) = list.iter().position(|(e, u)| e == &emoji && u == &from) {
                        list.remove(pos);
                    } else {
                        list.push((emoji, from));
                    }
                }
                NetToUi::Nudged { from } => {
                    self.nudge_time = 0.6; // 600ms shake
                    self.nudge_from = Some(from.clone());
                    self.show_toast(format!("💥 {} nudged you!", from), ToastKind::Info);
                    self.audio_manager.play(SoundEffect::MessageReceived);
                }
                NetToUi::Winked { from, emoji } => {
                    self.wink_animation = Some((emoji.clone(), 0.0, 0.0, from.clone()));
                    self.show_toast(format!("{} {} winked at you!", emoji, from), ToastKind::Info);
                    self.audio_manager.play(SoundEffect::MessageReceived);
                }
                NetToUi::ProfileData { username, bio, status, joined, avatar_url } => {
                    self.profile_cache.insert(username.clone(), (bio, status, joined, avatar_url));
                    if self.viewing_profile.as_ref() == Some(&username) {
                        // Profile modal is open, it will refresh automatically
                    }
                }
            }
        }
    }

    fn send_connect(&mut self) {
        if self.auth_mode == AuthMode::Register && self.password != self.confirm_password {
            self.status = "Passwords do not match".to_string();
            self.show_toast(self.status.clone(), ToastKind::Error);
            return;
        }
        self.logging_in = true;
        self.login_error = false;
        self.login_error_time = 0.0;
        self.login_success = false;
        self.login_success_time = 0.0;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.login_started_at = Some(Instant::now());
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.login_started_at = Some(0);  // Start frame count at 0
            self.login_frame_count = 0;
        }
        let safe_username = Self::sanitize_username(&self.username);
        self.status = format!("Logging in as {}...", safe_username);
        let mut url = self.server_url.trim().to_string();
        if let Some(stripped) = url.strip_prefix("https://") {
            url = format!("wss://{stripped}");
        } else if let Some(stripped) = url.strip_prefix("http://") {
            url = format!("ws://{stripped}");
        } else if !url.starts_with("ws://") && !url.starts_with("wss://") {
            // Default to ws:// for localhost, wss:// for everything else
            let is_local = url.starts_with("localhost") || url.starts_with("127.0.0.1") || url.starts_with("0.0.0.0");
            if is_local {
                url = format!("ws://{url}");
            } else {
                url = format!("wss://{url}");
            }
        }
        self.server_url = url.clone();
        let _ = self.network.tx.send(UiToNet::Connect {
            url,
            username: safe_username,
            password: Self::sanitize_input(&self.password),
            mode: self.auth_mode,
        });
    }

    fn send_chat(&mut self) {
        let body = Self::sanitize_input(&self.chat_input);
        if body.is_empty() {
            return;
        }
        let selected = self.selected_target.clone();
        match selected {
            ChatTarget::Lobby => {
                let _ = self
                    .network
                    .tx
                    .send(UiToNet::SendChat { body: body.clone() });
            }
            ChatTarget::Direct(target) => {
                #[cfg(not(target_arch = "wasm32"))]
                let sent_encrypted = self.send_encrypted_dm(&target, &body);
                #[cfg(target_arch = "wasm32")]
                let sent_encrypted = false;

                if !sent_encrypted {
                    let _ = self.network.tx.send(UiToNet::SendDirect {
                        to: target.clone(),
                        body: body.clone(),
                    });
                }
                let display_body = if sent_encrypted {
                    format!("🔒 {}", body)
                } else {
                    body.clone()
                };
                let entry = self
                    .messages
                    .entry(ChatTarget::Direct(target.clone()))
                    .or_default();
                entry.push(ChatMessage {
                    from: self
                        .logged_in_user
                        .clone()
                        .unwrap_or_else(|| "Me".to_string()),
                    body: display_body,
                    at: Utc::now().to_rfc3339(),
                    id: None,
                    read_count: None,
                });
            }
            ChatTarget::Room(room_id) => {
                let _ = self.network.tx.send(UiToNet::SendRoomMessage {
                    room_id: room_id.clone(),
                    body: body.clone(),
                });
                let entry = self.messages.entry(ChatTarget::Room(room_id.clone())).or_default();
                entry.push(ChatMessage {
                    from: self
                        .logged_in_user
                        .clone()
                        .unwrap_or_else(|| "Me".to_string()),
                    body,
                    at: Utc::now().to_rfc3339(),
                    id: None,
                    read_count: None,
                });
            }
        }
        self.audio_manager.play(SoundEffect::MessageSent);
        self.chat_input.clear();
    }

    fn send_away(&mut self) {
        let away = if self.away_text.trim().is_empty() {
            None
        } else {
            Some(Self::sanitize_input(&self.away_text))
        };
        let _ = self.network.tx.send(UiToNet::SetAway { away });
    }

    /// Initiate E2E key exchange with a DM peer (native only)
    #[cfg(not(target_arch = "wasm32"))]
    fn initiate_e2e_static(&mut self, peer: &str) {
        use rand::rngs::OsRng;
        let our_secret = StaticSecret::random_from_rng(OsRng);
        let our_public = PublicKey::from(&our_secret);
        let pub_encoded = BASE64.encode(our_public.to_bytes());
        let secret_bytes: [u8; 32] = our_secret.to_bytes();
        self.e2e_pending_secret = Some((peer.to_string(), secret_bytes));
        let _ = self.network.tx.send(UiToNet::ExchangeKey {
            to: peer.to_string(),
            public_key: pub_encoded,
        });
        self.show_toast(format!("🔐 Initiating E2E with {}...", peer), ToastKind::Info);
    }

    /// Complete E2E key exchange when we receive a peer's public key
    #[cfg(not(target_arch = "wasm32"))]
    fn complete_e2e(&mut self, from: &str, their_pub_b64: &str) {
        use rand::rngs::OsRng;

        // If we have a pending secret for this peer, complete the DH
        if let Some((pending_peer, our_secret_bytes)) = self.e2e_pending_secret.take() {
            if pending_peer == from {
                if let Ok(their_pub_bytes) = BASE64.decode(their_pub_b64) {
                    if their_pub_bytes.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&their_pub_bytes);
                        let their_public = PublicKey::from(arr);
                        let our_secret = StaticSecret::from(our_secret_bytes);
                        let shared = our_secret.diffie_hellman(&their_public);
                        self.e2e_shared_secrets.insert(from.to_string(), *shared.as_bytes());
                        self.show_toast(format!("🔒 E2E enabled with {}", from), ToastKind::Success);
                        return;
                    }
                }
            } else {
                // Put it back if it was for a different peer
                self.e2e_pending_secret = Some((pending_peer, our_secret_bytes));
            }
        }

        // They initiated — respond with our public key and compute shared secret
        if let Ok(their_pub_bytes) = BASE64.decode(their_pub_b64) {
            if their_pub_bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&their_pub_bytes);
                let their_public = PublicKey::from(arr);
                let our_secret = StaticSecret::random_from_rng(OsRng);
                let our_public = PublicKey::from(&our_secret);
                let shared = our_secret.diffie_hellman(&their_public);
                self.e2e_shared_secrets.insert(from.to_string(), *shared.as_bytes());
                // Send our public key back
                let pub_encoded = BASE64.encode(our_public.to_bytes());
                let _ = self.network.tx.send(UiToNet::ExchangeKey {
                    to: from.to_string(),
                    public_key: pub_encoded,
                });
                self.show_toast(format!("🔒 E2E enabled with {}", from), ToastKind::Success);
            }
        }
    }

    /// Encrypt and send a DM using E2E if a shared secret exists
    #[cfg(not(target_arch = "wasm32"))]
    fn send_encrypted_dm(&mut self, to: &str, body: &str) -> bool {
        use rand::RngCore;
        let secret_bytes = match self.e2e_shared_secrets.get(to) {
            Some(b) => *b,
            None => return false,
        };
        let key = Key::from_slice(&secret_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        match cipher.encrypt(nonce, body.as_bytes()) {
            Ok(ciphertext) => {
                let mut payload = nonce_bytes.to_vec();
                payload.extend_from_slice(&ciphertext);
                let encoded = BASE64.encode(&payload);
                let _ = self.network.tx.send(UiToNet::SendEncryptedDirect {
                    to: to.to_string(),
                    encrypted_body: encoded,
                });
                true
            }
            Err(_) => false,
        }
    }

    /// Decrypt an incoming E2E DM
    #[cfg(not(target_arch = "wasm32"))]
    fn decrypt_dm(&self, from: &str, encrypted_body: &str) -> Option<String> {
        let secret_bytes = self.e2e_shared_secrets.get(from)?;
        let payload = BASE64.decode(encrypted_body).ok()?;
        if payload.len() < 12 {
            return None;
        }
        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let key = Key::from_slice(secret_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
        String::from_utf8(plaintext).ok()
    }

    fn send_moderation(&mut self, action: UiToNet) {
        let _ = self.network.tx.send(action);
    }

    fn select_target(&mut self, target: ChatTarget) {
        if self.selected_target == target {
            return;
        }
        self.selected_target = target.clone();
        self.unread_counts.remove(&target); // clear unread on open
        self.search_query.clear();
        self.search_in_progress = false;
        self.search_results.remove(&target);
        let _ = self.network.tx.send(UiToNet::FetchHistory { target });
    }

    fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.theme = match self.theme {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::MidnightAmber,
            Theme::MidnightAmber => Theme::Light,
        };
        apply_theme(ctx, self.theme);
    }

    fn show_toast(&mut self, text: String, kind: ToastKind) {
        self.toast = Some(Toast {
            text,
            kind,
            ttl: 3.0,
        });
    }

    fn draw_toast(&mut self, ctx: &egui::Context) {
        let Some(toast) = &mut self.toast else {
            return;
        };
        let dt = ctx.input(|i| i.stable_dt);
        toast.ttl -= dt as f32;
        if toast.ttl <= 0.0 {
            self.toast = None;
            return;
        }

        let (fill, stroke) = match toast.kind {
            ToastKind::Info => (
                egui::Color32::from_rgb(50, 50, 55),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(95, 95, 105)),
            ),
            ToastKind::Success => (
                egui::Color32::from_rgb(35, 85, 55),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 140, 95)),
            ),
            ToastKind::Error => (
                egui::Color32::from_rgb(110, 45, 45),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(170, 70, 70)),
            ),
        };

        egui::TopBottomPanel::top("toast_bar")
            .exact_height(32.0)
            .show(ctx, |ui| {
                let frame = egui::Frame::new()
                    .fill(fill)
                    .stroke(stroke)
                    .inner_margin(egui::Margin::symmetric(10.0 as i8, 6.0 as i8));
                frame.show(ui, |ui| {
                    ui.label(toast.text.clone());
                });
            });
    }
}

impl eframe::App for AolApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_theme(ctx, self.theme);

        // Nudge screen shake
        if self.nudge_time > 0.0 {
            let dt = ctx.input(|i| i.stable_dt).min(0.05);
            self.nudge_time = (self.nudge_time - dt).max(0.0);
            let shake = (self.nudge_time * 60.0).sin() * 6.0 * (self.nudge_time / 0.6);
            ctx.set_transform_layer(
                egui::LayerId::new(egui::Order::Background, egui::Id::new("shake")),
                egui::emath::TSTransform::from_translation(egui::vec2(shake, 0.0)),
            );
            ctx.request_repaint();
        }
        
        // Wink animation - emoji bounces across screen
        if let Some((emoji, x_pos, time, from)) = &mut self.wink_animation {
            let dt = ctx.input(|i| i.stable_dt).min(0.05);
            *time += dt;
            *x_pos += dt * 400.0; // Move 400 pixels per second
            
            let screen_rect = ctx.screen_rect();
            let y_pos = screen_rect.center().y + (*time * 3.0).sin() * 50.0; // Bounce up and down
            
            // Draw the emoji
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("wink_animation"),
            ));
            
            let size = 48.0 + (*time * 8.0).sin() * 8.0; // Pulse size
            painter.text(
                egui::pos2(*x_pos, y_pos),
                egui::Align2::CENTER_CENTER,
                emoji,
                egui::FontId::proportional(size),
                egui::Color32::WHITE,
            );
            
            // Clear animation after it goes off screen or after 2 seconds
            if *x_pos > screen_rect.right() + 50.0 || *time > 2.0 {
                self.wink_animation = None;
            } else {
                ctx.request_repaint();
            }
        }
        
        if self.startup_repaint_left > 0 {
            self.startup_repaint_left = self.startup_repaint_left.saturating_sub(1);
            ctx.request_repaint();
        }
        self.process_net_events();
        
        // On native, timeout login after 15 seconds
        #[cfg(not(target_arch = "wasm32"))]
        if self.logging_in {
            if let Some(start) = self.login_started_at {
                if start.elapsed().as_secs() >= 15 {
                    self.logging_in = false;
                    self.login_started_at = None;
                    self.status = "Connection timed out. Check the server URL.".to_string();
                    self.show_toast(self.status.clone(), ToastKind::Error);
                }
            }
        }

        // On web, timeout login after 3 seconds (web networking is stubbed)
        #[cfg(target_arch = "wasm32")]
        if self.logging_in && self.screen == Screen::SignIn {
            self.login_frame_count += 1;
            let timeout_frames = 180;  // ~3 seconds at 60 FPS
            if self.login_frame_count >= timeout_frames {
                // Auto-approve login on web
                self.screen = Screen::Chat;
                self.connected = true;
                self.logged_in_user = Some(self.username.trim().to_string());
                self.status = format!("Connected as {}", self.username.trim());
                self.logging_in = false;
                self.login_started_at = None;
                self.login_frame_count = 0;
                self.show_toast("Web version: Messages won't sync yet".to_string(), ToastKind::Info);
            } else {
                // Still waiting, keep requesting repaints
                ctx.request_repaint();
            }
        }
        
        // Always request repaint if we're logging in (to show spinner)
        if self.logging_in {
            ctx.request_repaint();
        }
        
        if self.show_background {
            self.draw_background(ctx);
        }
        self.draw_toast(ctx);

        match self.screen {
            Screen::SignIn => {
                // Advance animation timers
                let dt = ctx.input(|i| i.stable_dt).min(0.05);
                self.login_anim_time += dt;
                self.login_scanline_offset = (self.login_scanline_offset + dt * 60.0) % 8.0;
                ctx.request_repaint();

                let amber       = egui::Color32::from_rgb(240, 168,  58);
                let amber_dim   = egui::Color32::from_rgb( 90,  55,  10);
                let amber_bright= egui::Color32::from_rgb(255, 210, 100);
                let green_matrix= egui::Color32::from_rgb(  0, 200,  80);

                let (card_fill, card_stroke, text_color) = match self.theme {
                    Theme::Light => (
                        egui::Color32::from_rgb(255, 250, 235),
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(210, 200, 175)),
                        egui::Color32::from_rgb(35, 30, 25),
                    ),
                    Theme::Dark => (
                        egui::Color32::from_rgb(28, 26, 24),
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(90, 85, 80)),
                        egui::Color32::from_rgb(235, 225, 210),
                    ),
                    Theme::MidnightAmber => (
                        egui::Color32::from_rgba_unmultiplied(28, 22, 12, 230),
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 80, 20)),
                        egui::Color32::from_rgb(231, 220, 198),
                    ),
                };

                // ── top bar ──────────────────────────────────────────────
                egui::TopBottomPanel::top("signin_top").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(amber, "◈ AOL-Style Messenger");
                        ui.separator();
                        let bg_label = if self.show_background { "BG: On" } else { "BG: Off" };
                        if ui.button(bg_label).clicked() { self.show_background = !self.show_background; }
                        let label = match self.theme {
                            Theme::Light => "Dark Mode",
                            Theme::Dark  => "Midnight Amber",
                            Theme::MidnightAmber => "Light Mode",
                        };
                        if ui.button("Refresh UI").clicked() { ctx.request_repaint(); }
                        if ui.button(label).clicked() { self.toggle_theme(ctx); }
                    });
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    let panel_rect = ui.max_rect();

                    // ── Matrix rain background ────────────────────────────
                    {
                        let matrix_chars: Vec<char> =
                            "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@#$%&*<>?/\\|~^"
                            .chars().collect();
                        let col_w = 14.0_f32;
                        let num_cols = (panel_rect.width() / col_w).ceil() as usize;

                        if !self.matrix_initialized || self.matrix_cols.len() != num_cols {
                            self.matrix_cols = (0..num_cols).map(|i| {
                                let seed = (i * 7 + 3) as f32;
                                MatrixCol {
                                    x: panel_rect.left() + i as f32 * col_w + col_w * 0.5,
                                    y: -(seed % 300.0),
                                    speed: 40.0 + (seed * 13.7) % 80.0,
                                    chars: {
                                        let len = 6 + (i * 3 + 2) % 10;
                                        (0..len).map(|j| matrix_chars[(i * 3 + j * 7) % matrix_chars.len()]).collect()
                                    },
                                    head: 0,
                                    length: 6 + (i * 3 + 2) % 10,
                                }
                            }).collect();
                            self.matrix_initialized = true;
                        }

                        let painter = ui.painter();
                        let char_h = 14.0_f32;

                        for col in &mut self.matrix_cols {
                            col.y += col.speed * dt;
                            let total_h = col.length as f32 * char_h;
                            if col.y > panel_rect.bottom() + total_h {
                                col.y = panel_rect.top() - total_h;
                                // Shuffle chars
                                for (j, c) in col.chars.iter_mut().enumerate() {
                                    *c = matrix_chars[(col.head + j * 11) % matrix_chars.len()];
                                }
                                col.head = (col.head + 1) % matrix_chars.len();
                            }

                            for (j, ch) in col.chars.iter().enumerate() {
                                let cy = col.y + j as f32 * char_h;
                                if cy < panel_rect.top() - char_h || cy > panel_rect.bottom() { continue; }
                                // Head char is bright, trail fades
                                let alpha = if j == col.length - 1 {
                                    255
                                } else {
                                    let fade = j as f32 / col.length as f32;
                                    (fade * fade * 120.0) as u8
                                };
                                let color = if j == col.length - 1 {
                                    egui::Color32::from_rgba_unmultiplied(180, 255, 180, 255)
                                } else {
                                    egui::Color32::from_rgba_unmultiplied(0, 180, 60, alpha)
                                };
                                painter.text(
                                    egui::pos2(col.x, cy),
                                    egui::Align2::CENTER_CENTER,
                                    &ch.to_string(),
                                    egui::FontId::monospace(11.0),
                                    color,
                                );
                            }
                        }

                        // Scanlines on top of matrix
                        let scanline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 28);
                        let mut sy = panel_rect.top() + self.login_scanline_offset;
                        while sy < panel_rect.bottom() {
                            painter.line_segment(
                                [egui::pos2(panel_rect.left(), sy), egui::pos2(panel_rect.right(), sy)],
                                egui::Stroke::new(1.0, scanline_color),
                            );
                            sy += 8.0;
                        }
                    }

                    ui.vertical_centered(|ui| {
                        ui.add_space(12.0);

                        // ── Boot sequence (shown until done) ─────────────
                        let boot_lines = [
                            "AOL Desktop v9.0  (c) 1998 America Online, Inc.",
                            "Initializing TCP/IP stack................. OK",
                            "Loading winsock.dll........................ OK",
                            "Checking modem............................ FOUND",
                            "Allocating screen buffers.................. OK",
                            "Loading buddy list......................... OK",
                            "System ready.",
                        ];
                        let boot_total_time = boot_lines.len() as f32 * 0.38;

                        if !self.boot_done {
                            self.boot_line_timer += dt;
                            if self.boot_line_timer >= 0.38 {
                                self.boot_line_timer = 0.0;
                                self.boot_line += 1;
                                if self.boot_line >= boot_lines.len() {
                                    self.boot_done = true;
                                }
                            }

                            egui::Frame::new()
                                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 210))
                                .corner_radius(egui::CornerRadius::same(6.0 as u8))
                                .inner_margin(egui::Margin::same(16.0 as i8))
                                .show(ui, |ui| {
                                    ui.set_min_width(480.0);
                                    ui.set_max_width(480.0);
                                    for (i, line) in boot_lines.iter().enumerate() {
                                        if i > self.boot_line { break; }
                                        let color = if i == self.boot_line {
                                            amber_bright
                                        } else if line.ends_with("OK") {
                                            green_matrix
                                        } else if line.ends_with("FOUND") {
                                            green_matrix
                                        } else {
                                            amber_dim
                                        };
                                        ui.label(egui::RichText::new(*line).monospace().size(12.0).color(color));
                                    }
                                    // Blinking cursor on last line
                                    if !self.boot_done && ((self.login_anim_time * 4.0) as u32 % 2 == 0) {
                                        ui.label(egui::RichText::new("█").monospace().size(12.0).color(amber));
                                    }
                                });
                            return; // Don't show login form until boot done
                        }

                        // ── Big ASCII AOL logo ────────────────────────────
                        let logo_lines = [
                            r"   ___   ___  _     ",
                            r"  / _ \ / _ \| |    ",
                            r" | |_| | | | | |    ",
                            r"  \__,_|\___/|_|    ",
                            r"  Instant Messenger ",
                        ];
                        // Shimmer: each char gets a brightness based on wave
                        let shimmer_t = self.login_anim_time * 3.0;
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 160))
                            .corner_radius(egui::CornerRadius::same(4.0 as u8))
                            .inner_margin(egui::Margin::symmetric(20.0 as i8, 8.0 as i8))
                            .show(ui, |ui| {
                                ui.set_min_width(320.0);
                                for (row, line) in logo_lines.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        for (col, ch) in line.chars().enumerate() {
                                            let wave = ((shimmer_t - (col as f32 + row as f32 * 3.0) * 0.18).sin() * 0.5 + 0.5) as f32;
                                            let r = (amber_dim.r() as f32 + (amber_bright.r() as f32 - amber_dim.r() as f32) * wave) as u8;
                                            let g = (amber_dim.g() as f32 + (amber_bright.g() as f32 - amber_dim.g() as f32) * wave) as u8;
                                            let b = (amber_dim.b() as f32 + (amber_bright.b() as f32 - amber_dim.b() as f32) * wave) as u8;
                                            let color = egui::Color32::from_rgb(r, g, b);
                                            ui.label(egui::RichText::new(ch.to_string()).monospace().size(15.0).color(color));
                                        }
                                    });
                                }
                            });

                        ui.add_space(6.0);

                        // ── Typewriter tagline ────────────────────────────
                        let tagline = "\"You've Got Mail.  The world is online.\"";
                        let tw_speed = 22.0_f32;
                        // Only start typewriter after boot
                        let tw_elapsed = (self.login_anim_time - boot_total_time).max(0.0);
                        let tw_target = ((tw_elapsed * tw_speed) as usize).min(tagline.len());
                        if self.login_typewriter_pos < tw_target {
                            self.login_typewriter_pos = tw_target;
                        }
                        let visible_tag = &tagline[..self.login_typewriter_pos.min(tagline.len())];
                        let cursor_char = if self.login_typewriter_pos < tagline.len()
                            && ((self.login_anim_time * 3.0) as u32 % 2 == 0) { "▌" } else { " " };
                        ui.label(
                            egui::RichText::new(format!("{}{}", visible_tag, cursor_char))
                                .monospace().size(13.0).color(amber),
                        );

                        ui.add_space(10.0);

                        // ── Login card ────────────────────────────────────
                        egui::Frame::new()
                            .fill(card_fill)
                            .stroke(card_stroke)
                            .corner_radius(egui::CornerRadius::same(10.0 as u8))
                            .inner_margin(egui::Margin::same(20.0 as i8))
                            .show(ui, |ui| {
                                ui.set_max_width(360.0);

                                ui.horizontal(|ui| {
                                    if ui.selectable_label(self.auth_mode == AuthMode::Login, "Sign In").clicked() {
                                        self.auth_mode = AuthMode::Login;
                                    }
                                    if ui.selectable_label(self.auth_mode == AuthMode::Register, "Create Account").clicked() {
                                        self.auth_mode = AuthMode::Register;
                                    }
                                });
                                ui.add_space(10.0);
                                let user_resp = ui.add(egui::TextEdit::singleline(&mut self.username).hint_text("Screen name"));
                                if user_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    self.send_connect();
                                    self.modem_line = 0;
                                    self.modem_line_timer = 0.0;
                                    self.modem_char_pos = 0;
                                }
                                ui.horizontal(|ui| {
                                    let pw_resp = ui.add(
                                        egui::TextEdit::singleline(&mut self.password)
                                            .password(!self.show_password)
                                            .hint_text("Password"),
                                    );
                                    ui.checkbox(&mut self.show_password, "Show");
                                    // Submit on Enter from password field
                                    if pw_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                        self.send_connect();
                                        self.modem_line = 0;
                                        self.modem_line_timer = 0.0;
                                        self.modem_char_pos = 0;
                                    }
                                });
                                if self.auth_mode == AuthMode::Register {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.confirm_password)
                                                .password(!self.show_confirm_password)
                                                .hint_text("Confirm password"),
                                        );
                                        ui.checkbox(&mut self.show_confirm_password, "Show");
                                    });
                                }
                                ui.add(egui::TextEdit::singleline(&mut self.server_url).hint_text("Server URL"));
                                
                                ui.add_space(8.0);
                                ui.checkbox(&mut self.remember_me, "Remember me");
                                
                                ui.add_space(12.0);

                                let button_label = match self.auth_mode {
                                    AuthMode::Login    => "Sign On",
                                    AuthMode::Register => "Create Account",
                                };
                                if ui.button(button_label).clicked() {
                                    self.send_connect();
                                    // Reset modem animation
                                    self.modem_line = 0;
                                    self.modem_line_timer = 0.0;
                                    self.modem_char_pos = 0;
                                }

                                ui.add_space(10.0);

                                // ── Dial-up modem connecting animation ────
                                if self.logging_in {
                                    let modem_script = [
                                        "ATDT 1-800-827-6364",
                                        "CONNECT 56000",
                                        "Verifying credentials...",
                                        "Checking buddy list...",
                                        "Loading away messages...",
                                        "Welcome to AOL!",
                                    ];

                                    // Advance modem animation
                                    self.modem_line_timer += dt;
                                    let line_delay = 0.55_f32;
                                    if self.modem_line_timer >= line_delay && self.modem_line < modem_script.len() {
                                        let current_line = modem_script[self.modem_line];
                                        if self.modem_char_pos < current_line.len() {
                                            // Typewriter within line: advance a few chars per frame
                                            self.modem_char_pos = (self.modem_char_pos + 3).min(current_line.len());
                                        } else {
                                            // Line done, move to next
                                            self.modem_line += 1;
                                            self.modem_char_pos = 0;
                                            self.modem_line_timer = 0.0;
                                        }
                                    }

                                    egui::Frame::new()
                                        .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200))
                                        .corner_radius(egui::CornerRadius::same(4.0 as u8))
                                        .inner_margin(egui::Margin::same(10.0 as i8))
                                        .show(ui, |ui| {
                                            ui.set_min_width(320.0);
                                            for (i, line) in modem_script.iter().enumerate() {
                                                if i > self.modem_line { break; }
                                                let text = if i == self.modem_line {
                                                    // Partially typed current line
                                                    &line[..self.modem_char_pos.min(line.len())]
                                                } else {
                                                    line
                                                };
                                                let color = if i == 0 {
                                                    // AT command in dim white
                                                    egui::Color32::from_rgb(200, 200, 200)
                                                } else if i == 1 {
                                                    green_matrix
                                                } else if i == 2 && line.contains("credentials") {
                                                    // "Verifying credentials..." in yellow/amber
                                                    egui::Color32::from_rgb(255, 200, 80)
                                                } else if i == modem_script.len() - 1 {
                                                    amber_bright
                                                } else {
                                                    amber
                                                };
                                                ui.label(egui::RichText::new(text).monospace().size(12.0).color(color));
                                            }
                                            // Blinking cursor
                                            if (self.login_anim_time * 4.0) as u32 % 2 == 0 {
                                                ui.label(egui::RichText::new("█").monospace().size(12.0).color(amber));
                                            }
                                        });
                                } else if self.login_success {
                                    // Success state with green flash effect
                                    self.login_success_time += dt;
                                    
                                    // Flash green for 1 second
                                    let flash_intensity = if self.login_success_time < 0.8 {
                                        ((self.login_success_time * 10.0).sin() * 0.5 + 0.5) as f32
                                    } else {
                                        1.0
                                    };
                                    
                                    let success_color = egui::Color32::from_rgb(
                                        (50.0 * (1.0 - flash_intensity)) as u8,
                                        (180.0 + 75.0 * flash_intensity) as u8,
                                        (50.0 * (1.0 - flash_intensity)) as u8,
                                    );
                                    
                                    egui::Frame::new()
                                        .fill(egui::Color32::from_rgba_unmultiplied(0, 40, 0, (200.0 * flash_intensity) as u8))
                                        .stroke(egui::Stroke::new(2.0, success_color))
                                        .corner_radius(egui::CornerRadius::same(4.0 as u8))
                                        .inner_margin(egui::Margin::same(10.0 as i8))
                                        .show(ui, |ui| {
                                            ui.set_min_width(320.0);
                                            ui.label(egui::RichText::new("✓ AUTHENTICATION SUCCESSFUL").monospace().size(12.0).color(success_color).strong());
                                            ui.label(egui::RichText::new("Welcome back!").monospace().size(11.0).color(success_color));
                                            ui.add_space(4.0);
                                            ui.label(egui::RichText::new("Loading your buddy list...").monospace().size(10.0).color(success_color));
                                        });
                                    
                                    // Switch to chat screen after 1.2 seconds
                                    if self.login_success_time > 1.2 {
                                        self.screen = Screen::Chat;
                                        self.auth_mode = AuthMode::Login;
                                        self.confirm_password.clear();
                                        self.password.clear();
                                        self.login_success = false;
                                        self.login_success_time = 0.0;
                                    }
                                    
                                    ctx.request_repaint();
                                } else if self.login_error {
                                    // Error state with red flash effect
                                    self.login_error_time += dt;
                                    
                                    // Flash red for first 2 seconds, then stay solid red
                                    let flash_intensity = if self.login_error_time < 2.0 {
                                        ((self.login_error_time * 8.0).sin() * 0.3 + 0.7) as f32
                                    } else {
                                        1.0  // Stay solid after flashing
                                    };
                                    
                                    let error_color = egui::Color32::from_rgb(
                                        (200.0 + 55.0 * flash_intensity) as u8,
                                        (50.0 * (1.0 - flash_intensity * 0.5)) as u8,
                                        (50.0 * (1.0 - flash_intensity * 0.5)) as u8,
                                    );
                                    
                                    egui::Frame::new()
                                        .fill(egui::Color32::from_rgba_unmultiplied(40, 0, 0, 200))
                                        .stroke(egui::Stroke::new(2.0, error_color))
                                        .corner_radius(egui::CornerRadius::same(4.0 as u8))
                                        .inner_margin(egui::Margin::same(10.0 as i8))
                                        .show(ui, |ui| {
                                            ui.set_min_width(320.0);
                                            ui.label(egui::RichText::new("❌ AUTHENTICATION FAILED").monospace().size(12.0).color(error_color).strong());
                                            ui.label(egui::RichText::new(&self.status).monospace().size(11.0).color(error_color));
                                        });
                                    
                                    // Keep requesting repaint only during flash animation
                                    if self.login_error_time < 2.0 {
                                        ctx.request_repaint();
                                    }
                                } else {
                                    ui.colored_label(text_color, format!("Status: {}", self.status));
                                }
                            });
                    });
                });
            }
            Screen::Chat => {
                // Global keyboard shortcuts
                ctx.input(|i| {
                    // Ctrl+K or Cmd+K: Focus search (jump to chat)
                    if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::K) {
                        self.search_query.clear();
                        // Focus will be handled by the search field
                    }
                    
                    // Ctrl+D or Cmd+D: Open DM input
                    if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::D) {
                        // Focus DM target field (will be handled by the field itself)
                    }
                    
                    // Ctrl+F or Cmd+F: Focus search
                    if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::F) {
                        // Focus search field
                    }
                    
                    // Escape: Close modals
                    if i.key_pressed(egui::Key::Escape) {
                        self.show_add_friend_modal = false;
                        self.show_friend_requests_modal = false;
                        self.show_avatar_modal = false;
                        self.viewing_profile = None;
                    }
                });

                egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
                    // First row: Title, status, user info, and main actions
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.heading("AOL Messenger");
                        ui.add_space(8.0);
                        ui.separator();
                        
                        if let Some(name) = &self.logged_in_user {
                            ui.label(format!("👤 {name}"));
                        }

                        ui.separator();
                        let online_count = self.buddies.iter()
                            .filter(|b| Some(&b.username) != self.logged_in_user.as_ref() && b.away.is_none())
                            .count();
                        let total_buddies = self.buddies.iter()
                            .filter(|b| Some(&b.username) != self.logged_in_user.as_ref())
                            .count();
                        ui.label(format!("👥 {}/{}", online_count, total_buddies));
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Logout").clicked() {
                                // Clear saved credentials
                                let _ = std::fs::remove_file(self.credentials_file());
                                
                                // Disconnect and return to login
                                let _ = self.network.tx.send(UiToNet::Disconnect);
                                self.screen = Screen::SignIn;
                                self.logged_in_user = None;
                                self.selected_target = ChatTarget::Lobby;
                                self.username.clear();
                                self.password.clear();
                                self.remember_me = false;
                            }

                            // Audio settings menu
                            ui.menu_button("🔊", |ui| {
                                ui.checkbox(&mut self.sound_enabled, "Enable sound effects");

                                ui.horizontal(|ui| {
                                    ui.label("Volume:");
                                    ui.add(egui::Slider::new(&mut self.sound_volume, 0.0..=1.0)
                                        .show_value(false));
                                });

                                // Apply settings to audio manager
                                self.audio_manager.set_enabled(self.sound_enabled);
                                self.audio_manager.set_volume(self.sound_volume);

                                ui.separator();
                                ui.label("Test Sounds:");
                                if ui.button("🔔 Sign On").clicked() {
                                    self.audio_manager.play(SoundEffect::BuddySignOn);
                                }
                                if ui.button("🚪 Sign Off").clicked() {
                                    self.audio_manager.play(SoundEffect::BuddySignOff);
                                }
                                if ui.button("💬 Message").clicked() {
                                    self.audio_manager.play(SoundEffect::MessageReceived);
                                }
                            });

                            // Show friend requests button with pending count
                            let pending_count = self.pending_friend_requests.len();
                            let fr_label = if pending_count > 0 {
                                format!("📬 ({})", pending_count)
                            } else {
                                "📬".to_string()
                            };
                            if ui.button(fr_label).on_hover_text("Friend Requests").clicked() {
                                self.show_friend_requests_modal = true;
                            }

                            if ui.button("➕").on_hover_text("Add Friend").clicked() {
                                self.show_add_friend_modal = true;
                            }
                        });
                    });
                    
                    // Second row: Away message and custom status
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.label("Away:");
                        ui.add(egui::TextEdit::singleline(&mut self.away_text)
                            .hint_text("Be right back...")
                            .desired_width(140.0));
                        if ui.small_button("Set").clicked() {
                            self.send_away();
                        }
                        
                        ui.separator();
                        
                        ui.label("Status:");
                        ui.add(egui::TextEdit::singleline(&mut self.custom_status)
                            .hint_text("🎮 Playing Halo")
                            .desired_width(140.0));
                        if ui.small_button("Set").clicked() {
                            let status = if self.custom_status.trim().is_empty() {
                                None
                            } else {
                                Some(Self::sanitize_input(&self.custom_status))
                            };
                            let _ = self.network.tx.send(UiToNet::SetStatus { status });
                            self.custom_status.clear();
                        }
                    });
                    // Modal for Add Friend
                    if self.show_add_friend_modal {
                        egui::Window::new("Add Friend")
                            .collapsible(false)
                            .resizable(false)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                ui.label("Enter friend's screen name:");
                                let text_edit = ui.add(egui::TextEdit::singleline(&mut self.add_friend_name).hint_text("Screen name"));
                                
                                // Auto-focus the text input when modal opens
                                text_edit.request_focus();
                                
                                // Submit on Enter key
                                if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    self.send_add_friend();
                                    self.show_add_friend_modal = false;
                                }
                                
                                ui.horizontal(|ui| {
                                    if ui.button("Add").clicked() {
                                        self.send_add_friend();
                                        self.show_add_friend_modal = false;
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.show_add_friend_modal = false;
                                        self.add_friend_name.clear();
                                    }
                                });
                                
                                // Close on Escape
                                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    self.show_add_friend_modal = false;
                                    self.add_friend_name.clear();
                                }
                            });
                    }
                    // Modal for Friend Requests
                    if self.show_friend_requests_modal {
                        egui::Window::new("Friend Requests")
                            .collapsible(false)
                            .resizable(true)
                            .default_size([400.0, 300.0])
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                if self.pending_friend_requests.is_empty() {
                                    ui.label("No pending friend requests");
                                } else {
                                    ui.label(format!("You have {} pending request(s):", self.pending_friend_requests.len()));
                                    ui.separator();
                                    
                                    // Display each pending request
                                    let requests = self.pending_friend_requests.clone();  // Clone to avoid borrow issues
                                    for username in requests {
                                        ui.horizontal(|ui| {
                                            ui.label(&username);
                                            if ui.button("✓ Accept").clicked() {
                                                self.accept_friend_request(username.clone());
                                            }
                                            if ui.button("✗ Decline").clicked() {
                                                self.decline_friend_request(username.clone());
                                            }
                                        });
                                    }
                                }
                                
                                ui.separator();
                                if ui.button("Close").clicked() {
                                    self.show_friend_requests_modal = false;
                                }
                            });
                    }

                    // Modal for Avatar Upload
                    if self.show_avatar_modal {
                        egui::Window::new("Change Avatar")
                            .collapsible(false)
                            .resizable(false)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                ui.label("Enter image URL or emoji:");
                                ui.add(egui::TextEdit::singleline(&mut self.avatar_upload_path).hint_text("https://... or 🎮"));
                                
                                ui.horizontal(|ui| {
                                    if ui.button("Set Avatar").clicked() {
                                        if !self.avatar_upload_path.trim().is_empty() {
                                            let _ = self.network.tx.send(UiToNet::SetAvatar { 
                                                avatar_data: self.avatar_upload_path.clone() 
                                            });
                                            self.show_toast("Avatar updated!".to_string(), ToastKind::Success);
                                            self.show_avatar_modal = false;
                                            self.avatar_upload_path.clear();
                                        }
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.show_avatar_modal = false;
                                        self.avatar_upload_path.clear();
                                    }
                                });
                                
                                ui.separator();
                                ui.label("Quick emoji avatars:");
                                ui.horizontal_wrapped(|ui| {
                                    let emojis = ["😀", "😎", "🤖", "👾", "🎮", "🎨", "🎭", "🎪", "🎯", "🎲", "🎸", "🎹", "🚀", "🌟", "⭐", "💎"];
                                    for emoji in emojis {
                                        if ui.button(emoji).clicked() {
                                            let _ = self.network.tx.send(UiToNet::SetAvatar { 
                                                avatar_data: emoji.to_string() 
                                            });
                                            self.show_toast("Avatar updated!".to_string(), ToastKind::Success);
                                            self.show_avatar_modal = false;
                                        }
                                    }
                                });
                            });
                    }

                    // Modal for creating new buddy group
                    if self.show_group_modal {
                        egui::Window::new("Create Buddy Group")
                            .collapsible(false)
                            .resizable(false)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                ui.label("Group name:");
                                let text_edit = ui.add(egui::TextEdit::singleline(&mut self.new_group_name).hint_text("Work, Friends, etc."));
                                text_edit.request_focus();
                                
                                ui.horizontal(|ui| {
                                    if ui.button("Create").clicked() || (text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                        let group_name = self.new_group_name.trim().to_string();
                                        if !group_name.is_empty() && !self.buddy_groups.contains_key(&group_name) {
                                            self.buddy_groups.insert(group_name.clone(), Vec::new());
                                            // Add the buddy if one was selected
                                            if let Some(ref username) = self.group_modal_username {
                                                if let Some(group) = self.buddy_groups.get_mut(&group_name) {
                                                    group.push(username.clone());
                                                }
                                            }
                                            self.save_buddy_groups();
                                            self.show_toast(format!("Created group: {}", group_name), ToastKind::Success);
                                            self.show_group_modal = false;
                                            self.new_group_name.clear();
                                            self.group_modal_username = None;
                                        }
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.show_group_modal = false;
                                        self.new_group_name.clear();
                                        self.group_modal_username = None;
                                    }
                                });
                            });
                    }

                    // Profile modal
                    if let Some(ref username) = self.viewing_profile.clone() {
                        let mut close_modal = false;
                        egui::Window::new(format!("Profile: {}", username))
                            .collapsible(false)
                            .resizable(false)
                            .default_size([400.0, 300.0])
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                ui.vertical_centered(|ui| {
                                    // Show avatar if available, otherwise show colored circle
                                    if let Some((_, _, _, Some(avatar_url))) = self.profile_cache.get(username) {
                                        if !avatar_url.is_empty() {
                                            // Display avatar image (base64 or URL)
                                            ui.label(egui::RichText::new("🖼️").size(64.0));
                                            ui.label(egui::RichText::new("[Avatar Image]").small().italics());
                                        } else {
                                            // Fallback to colored circle
                                            let color = username_to_color(username);
                                            let initials = get_initials(username);
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(64.0, 64.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().circle_filled(rect.center(), 32.0, color);
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                &initials,
                                                egui::FontId::proportional(28.0),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                    } else {
                                        // Fallback to colored circle
                                        let color = username_to_color(username);
                                        let initials = get_initials(username);
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(64.0, 64.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().circle_filled(rect.center(), 32.0, color);
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            &initials,
                                            egui::FontId::proportional(28.0),
                                            egui::Color32::WHITE,
                                        );
                                    }
                                    
                                    // Add "Change Avatar" button for own profile
                                    if self.logged_in_user.as_ref() == Some(username) {
                                        if ui.button("📷 Change Avatar").clicked() {
                                            self.show_avatar_modal = true;
                                        }
                                    }
                                    
                                    ui.add_space(8.0);
                                    ui.heading(username);
                                });

                                ui.separator();

                                if let Some((bio, status, joined, _avatar)) = self.profile_cache.get(username).cloned() {
                                    if let Some(ref status_text) = status {
                                        ui.label(egui::RichText::new(status_text).italics().color(egui::Color32::GRAY));
                                        ui.add_space(4.0);
                                    }

                                    ui.label(egui::RichText::new("Bio:").strong());
                                    
                                    // Allow editing own bio
                                    if self.logged_in_user.as_ref() == Some(username) {
                                        if self.bio_editing {
                                            let bio_response = ui.add(egui::TextEdit::multiline(&mut self.bio_edit).desired_width(f32::INFINITY));
                                            bio_response.request_focus();
                                            
                                            ui.horizontal(|ui| {
                                                if ui.button("Save").clicked() {
                                                    let _ = self.network.tx.send(UiToNet::SetBio { bio: self.bio_edit.clone() });
                                                    self.bio_editing = false;
                                                    // Update cache
                                                    if let Some(entry) = self.profile_cache.get_mut(username) {
                                                        entry.0 = self.bio_edit.clone();
                                                    }
                                                }
                                                if ui.button("Cancel").clicked() {
                                                    self.bio_editing = false;
                                                }
                                            });
                                        } else {
                                            if bio.is_empty() {
                                                ui.label(egui::RichText::new("No bio set").italics().color(egui::Color32::GRAY));
                                            } else {
                                                ui.label(&bio);
                                            }
                                            if ui.button("Edit Bio").clicked() {
                                                self.bio_edit = bio.clone();
                                                self.bio_editing = true;
                                            }
                                        }
                                    } else {
                                        if bio.is_empty() {
                                            ui.label(egui::RichText::new("No bio set").italics().color(egui::Color32::GRAY));
                                        } else {
                                            ui.label(&bio);
                                        }
                                    }

                                    ui.add_space(8.0);
                                    ui.label(format!("Joined: {}", joined));
                                } else {
                                    ui.label("Loading profile...");
                                }

                                ui.separator();
                                if ui.button("Close").clicked() {
                                    close_modal = true;
                                    self.bio_editing = false;
                                }
                            });
                        
                        if close_modal {
                            self.viewing_profile = None;
                        }
                    }
                });

                egui::SidePanel::left("buddy_list")
                    .resizable(false)
                    .min_width(200.0)
                    .show(ctx, |ui| {
                        ui.heading("Buddy List");
                        ui.separator();
                        // Lobby with unread badge
                        ui.horizontal(|ui| {
                            let selected = self.selected_target == ChatTarget::Lobby;
                            if ui.selectable_label(selected, "Lobby").clicked() {
                                self.select_target(ChatTarget::Lobby);
                            }
                            if let Some(&count) = self.unread_counts.get(&ChatTarget::Lobby) {
                                if count > 0 {
                                    ui.label(egui::RichText::new(format!("●{}", count))
                                        .small().color(egui::Color32::from_rgb(220, 60, 60)));
                                }
                            }
                        });
                        ui.add_space(8.0);
                        ui.label("Recent DMs");
                        let recent_threads = self.recent_threads.clone();
                        for name in recent_threads {
                            let target = ChatTarget::Direct(name.clone());
                            ui.horizontal(|ui| {
                                if ui.selectable_label(self.selected_target == target, &name).clicked() {
                                    self.select_target(ChatTarget::Direct(name));
                                }
                                if let Some(&count) = self.unread_counts.get(&target) {
                                    if count > 0 {
                                        ui.label(egui::RichText::new(format!("●{}", count))
                                            .small().color(egui::Color32::from_rgb(220, 60, 60)));
                                    }
                                }
                            });
                        }
                        if self.recent_threads.is_empty() {
                            ui.label("No recent threads.");
                        }
                        ui.add_space(8.0);
                        ui.label("Direct messages");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.dm_target)
                                    .hint_text("Screen name"),
                            );
                            if ui.button("Start DM").clicked() {
                                let target = self.dm_target.trim();
                                if !target.is_empty() {
                                    self.select_target(ChatTarget::Direct(target.to_string()));
                                    self.dm_target.clear();
                                }
                            }
                        });
                        ui.add_space(8.0);

                        // Separate buddies by status (exclude self)
                        let buddies = self.buddies.clone();
                        let (online, away): (Vec<_>, Vec<_>) = buddies
                            .iter()
                            .filter(|b| Some(&b.username) != self.logged_in_user.as_ref())
                            .partition(|b| b.away.is_none());

                        // Online buddies section
                        if !online.is_empty() {
                            ui.colored_label(
                                egui::Color32::from_rgb(0, 200, 0),
                                format!("🟢 Online ({})", online.len())
                            );
                            ui.separator();

                            for buddy in &online {
                                ui.horizontal(|ui| {
                                    // Small profile circle or avatar
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(24.0, 24.0),
                                        egui::Sense::click(),
                                    );
                                    
                                    // Show avatar if available, otherwise colored circle
                                    if let Some(ref avatar_url) = buddy.avatar_url {
                                        if !avatar_url.is_empty() {
                                            // If it's a single emoji, display it
                                            if avatar_url.chars().count() <= 2 {
                                                ui.painter().text(
                                                    rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    avatar_url,
                                                    egui::FontId::proportional(20.0),
                                                    egui::Color32::WHITE,
                                                );
                                            } else {
                                                // For URLs, show a placeholder icon
                                                ui.painter().circle_filled(rect.center(), 12.0, egui::Color32::from_rgb(100, 100, 100));
                                                ui.painter().text(
                                                    rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    "🖼️",
                                                    egui::FontId::proportional(12.0),
                                                    egui::Color32::WHITE,
                                                );
                                            }
                                        } else {
                                            // Fallback to colored circle
                                            let color = username_to_color(&buddy.username);
                                            let initials = get_initials(&buddy.username);
                                            ui.painter().circle_filled(rect.center(), 12.0, color);
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                &initials,
                                                egui::FontId::proportional(10.0),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                    } else {
                                        // Fallback to colored circle
                                        let color = username_to_color(&buddy.username);
                                        let initials = get_initials(&buddy.username);
                                        ui.painter().circle_filled(rect.center(), 12.0, color);
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            &initials,
                                            egui::FontId::proportional(10.0),
                                            egui::Color32::WHITE,
                                        );
                                    }

                                    // Click profile circle to view profile
                                    if response.clicked() {
                                        self.viewing_profile = Some(buddy.username.clone());
                                        let _ = self.network.tx.send(UiToNet::FetchProfile { username: buddy.username.clone() });
                                    }

                                    let target = ChatTarget::Direct(buddy.username.clone());
                                    let username_response = ui.selectable_label(
                                        self.selected_target == target,
                                        &buddy.username
                                    );
                                    
                                    // Single click to select, double-click to open DM
                                    if username_response.clicked() {
                                        self.select_target(ChatTarget::Direct(buddy.username.clone()));
                                    }
                                    if username_response.double_clicked() {
                                        self.select_target(ChatTarget::Direct(buddy.username.clone()));
                                    }
                                    
                                    // Right-click context menu
                                    username_response.context_menu(|ui| {
                                        if ui.button("💬 Send DM").clicked() {
                                            self.select_target(ChatTarget::Direct(buddy.username.clone()));
                                            ui.close_menu();
                                        }
                                        if ui.button("👤 View Profile").clicked() {
                                            self.viewing_profile = Some(buddy.username.clone());
                                            let _ = self.network.tx.send(UiToNet::FetchProfile { username: buddy.username.clone() });
                                            ui.close_menu();
                                        }
                                        if ui.button("💥 Nudge").clicked() {
                                            let _ = self.network.tx.send(UiToNet::Nudge { to: buddy.username.clone() });
                                            self.show_toast(format!("Nudged {}!", buddy.username), ToastKind::Info);
                                            ui.close_menu();
                                        }
                                        ui.menu_button("😉 Wink", |ui| {
                                            let emojis = ["😉", "😘", "👋", "💖", "✨", "🎉", "👍", "🔥"];
                                            for emoji in emojis {
                                                if ui.button(emoji).clicked() {
                                                    let _ = self.network.tx.send(UiToNet::Wink { 
                                                        to: buddy.username.clone(), 
                                                        emoji: emoji.to_string() 
                                                    });
                                                    self.show_toast(format!("Winked {} at {}!", emoji, buddy.username), ToastKind::Info);
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                    });

                                    // Show custom status if set
                                    if let Some(ref status) = buddy.status {
                                        ui.label(
                                            egui::RichText::new(status)
                                                .small()
                                                .color(egui::Color32::GRAY)
                                        );
                                    }
                                    
                                    // Show idle time if > 5 minutes
                                    if let Some(idle_secs) = buddy.last_activity {
                                        if idle_secs > 300 { // 5 minutes
                                            let idle_str = format_idle_time(idle_secs);
                                            ui.label(
                                                egui::RichText::new(format!("(Idle {})", idle_str))
                                                    .small()
                                                    .italics()
                                                    .color(egui::Color32::from_rgb(150, 150, 150))
                                            );
                                        }
                                    }
                                });
                            }
                        }

                        ui.add_space(8.0);

                        // Away buddies section
                        if !away.is_empty() {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 200, 0),
                                format!("🟡 Away ({})", away.len())
                            );
                            ui.separator();

                            for buddy in &away {
                                ui.horizontal(|ui| {
                                    // Small profile circle or avatar
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(24.0, 24.0),
                                        egui::Sense::click(),
                                    );
                                    
                                    // Show avatar if available, otherwise colored circle
                                    if let Some(ref avatar_url) = buddy.avatar_url {
                                        if !avatar_url.is_empty() {
                                            // If it's a single emoji, display it
                                            if avatar_url.chars().count() <= 2 {
                                                ui.painter().text(
                                                    rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    avatar_url,
                                                    egui::FontId::proportional(20.0),
                                                    egui::Color32::WHITE,
                                                );
                                            } else {
                                                // For URLs, show a placeholder icon
                                                ui.painter().circle_filled(rect.center(), 12.0, egui::Color32::from_rgb(100, 100, 100));
                                                ui.painter().text(
                                                    rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    "🖼️",
                                                    egui::FontId::proportional(12.0),
                                                    egui::Color32::WHITE,
                                                );
                                            }
                                        } else {
                                            // Fallback to colored circle
                                            let color = username_to_color(&buddy.username);
                                            let initials = get_initials(&buddy.username);
                                            ui.painter().circle_filled(rect.center(), 12.0, color);
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                &initials,
                                                egui::FontId::proportional(10.0),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                    } else {
                                        // Fallback to colored circle
                                        let color = username_to_color(&buddy.username);
                                        let initials = get_initials(&buddy.username);
                                        ui.painter().circle_filled(rect.center(), 12.0, color);
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            &initials,
                                            egui::FontId::proportional(10.0),
                                            egui::Color32::WHITE,
                                        );
                                    }

                                    // Click profile circle to view profile
                                    if response.clicked() {
                                        self.viewing_profile = Some(buddy.username.clone());
                                        let _ = self.network.tx.send(UiToNet::FetchProfile { username: buddy.username.clone() });
                                    }

                                    let target = ChatTarget::Direct(buddy.username.clone());
                                    let username_response = ui.selectable_label(
                                        self.selected_target == target,
                                        &buddy.username
                                    );
                                    
                                    // Single click to select, double-click to open DM
                                    if username_response.clicked() {
                                        self.select_target(ChatTarget::Direct(buddy.username.clone()));
                                    }
                                    if username_response.double_clicked() {
                                        self.select_target(ChatTarget::Direct(buddy.username.clone()));
                                    }
                                    
                                    // Right-click context menu
                                    username_response.context_menu(|ui| {
                                        if ui.button("💬 Send DM").clicked() {
                                            self.select_target(ChatTarget::Direct(buddy.username.clone()));
                                            ui.close_menu();
                                        }
                                        if ui.button("👤 View Profile").clicked() {
                                            self.viewing_profile = Some(buddy.username.clone());
                                            let _ = self.network.tx.send(UiToNet::FetchProfile { username: buddy.username.clone() });
                                            ui.close_menu();
                                        }
                                        if ui.button("💥 Nudge").clicked() {
                                            let _ = self.network.tx.send(UiToNet::Nudge { to: buddy.username.clone() });
                                            self.show_toast(format!("Nudged {}!", buddy.username), ToastKind::Info);
                                            ui.close_menu();
                                        }
                                        ui.menu_button("😉 Wink", |ui| {
                                            let emojis = ["😉", "😘", "👋", "💖", "✨", "🎉", "👍", "🔥"];
                                            for emoji in emojis {
                                                if ui.button(emoji).clicked() {
                                                    let _ = self.network.tx.send(UiToNet::Wink { 
                                                        to: buddy.username.clone(), 
                                                        emoji: emoji.to_string() 
                                                    });
                                                    self.show_toast(format!("Winked {} at {}!", emoji, buddy.username), ToastKind::Info);
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                    });

                                    if let Some(ref away_msg) = buddy.away {
                                        ui.label(
                                            egui::RichText::new(format!("({})", away_msg))
                                                .italics()
                                                .small()
                                                .color(egui::Color32::GRAY)
                                        );
                                    }

                                    // Show custom status if set
                                    if let Some(ref status) = buddy.status {
                                        ui.label(
                                            egui::RichText::new(status)
                                                .small()
                                                .color(egui::Color32::GRAY)
                                        );
                                    }
                                    
                                    // Show idle time if > 5 minutes
                                    if let Some(idle_secs) = buddy.last_activity {
                                        if idle_secs > 300 { // 5 minutes
                                            let idle_str = format_idle_time(idle_secs);
                                            ui.label(
                                                egui::RichText::new(format!("(Idle {})", idle_str))
                                                    .small()
                                                    .italics()
                                                    .color(egui::Color32::from_rgb(150, 150, 150))
                                            );
                                        }
                                    }
                                });
                            }
                        }

                        if online.is_empty() && away.is_empty() {
                            ui.label("No buddies online.");
                        }
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    let heading = match &self.selected_target {
                        ChatTarget::Lobby => "Chat Log - Lobby".to_string(),
                        ChatTarget::Direct(name) => format!("Chat Log - {name}"),
                        ChatTarget::Room(room_id) => format!("Chat Log - Room {}", room_id),
                    };
                    ui.heading(heading);
                    ui.separator();
                    ui.horizontal(|ui| {
                        let direct_name = match &self.selected_target {
                            ChatTarget::Direct(name) => Some(name.clone()),
                            _ => None,
                        };
                        if let Some(name) = direct_name {
                            if ui.button("Block").clicked() {
                                self.send_moderation(UiToNet::Block { username: name.clone() });
                            }
                            if ui.button("Unblock").clicked() {
                                self.send_moderation(UiToNet::Unblock { username: name.clone() });
                            }
                            if ui.button("Mute").clicked() {
                                self.send_moderation(UiToNet::Mute { username: name.clone() });
                            }
                            if ui.button("Unmute").clicked() {
                                self.send_moderation(UiToNet::Unmute { username: name.clone() });
                            }
                            // E2E encryption toggle (native only)
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let has_e2e = self.e2e_shared_secrets.contains_key(&name);
                                let e2e_label = if has_e2e { "🔒 E2E On" } else { "🔓 Enable E2E" };
                                if ui.button(e2e_label).clicked() && !has_e2e {
                                    self.initiate_e2e_static(&name.clone());
                                }
                            }
                            // Nudge button
                            if ui.button("💥 Nudge").clicked() {
                                let _ = self.network.tx.send(UiToNet::Nudge { to: name.clone() });
                                self.show_toast(format!("Nudged {}!", name), ToastKind::Info);
                            }
                            ui.add(
                                egui::TextEdit::singleline(&mut self.report_reason)
                                    .hint_text("Report reason")
                                    .desired_width(160.0),
                            );
                            if ui.button("Report").clicked() {
                                if !self.report_reason.trim().is_empty() {
                                    self.send_moderation(UiToNet::Report {
                                        username: name.clone(),
                                        reason: self.report_reason.trim().to_string(),
                                    });
                                    self.report_reason.clear();
                                }
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        let search_response = ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text("Find messages")
                                .desired_width(200.0),
                        );
                        let search_clicked = ui.button("Search Server").clicked();
                        let search_enter = search_response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if search_clicked || search_enter {
                            let query = self.search_query.trim();
                            if !query.is_empty() {
                                self.search_in_progress = true;
                                let _ = self.network.tx.send(UiToNet::Search {
                                    target: self.selected_target.clone(),
                                    query: query.to_string(),
                                });
                            }
                        }
                        if ui.button("Clear").clicked() {
                            self.search_query.clear();
                            self.search_in_progress = false;
                            self.search_results.remove(&self.selected_target);
                        }
                        if self.search_in_progress {
                            ui.label("Searching...");
                        }
                    });
                    egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                        let query = self.search_query.trim();
                        let mut using_search = false;
                        let messages = if !query.is_empty() {
                            if let Some(result) = self.search_results.get(&self.selected_target) {
                                if result.query == query {
                                    using_search = true;
                                    result.messages.clone()
                                } else {
                                    Vec::new()
                                }
                            } else {
                                Vec::new()
                            }
                        } else {
                            self.messages
                                .get(&self.selected_target)
                                .cloned()
                                .unwrap_or_default()
                        };

                        for message in messages {
                            ui.horizontal(|ui| {
                                // Profile circle with initials
                                let color = username_to_color(&message.from);
                                let initials = get_initials(&message.from);

                                // Draw circle
                                let (rect, _response) = ui.allocate_exact_size(
                                    egui::vec2(32.0, 32.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().circle_filled(
                                    rect.center(),
                                    16.0,
                                    color,
                                );
                                // Draw initials
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    &initials,
                                    egui::FontId::proportional(14.0),
                                    egui::Color32::WHITE,
                                );

                                // Message content
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&message.from)
                                                .strong()
                                                .color(color)
                                        );
                                        let relative = format_relative_time(&message.at);
                                        if !relative.is_empty() {
                                            let full_time = format_full_timestamp(&message.at);
                                            ui.label(
                                                egui::RichText::new(format!("• {}", relative))
                                                    .small()
                                                    .color(egui::Color32::GRAY)
                                            ).on_hover_text(full_time);
                                        }
                                    });
                                    // Convert emoticons and display message
                                    let body_with_emoji = convert_emoticons(&message.body);
                                    // Render with clickable links
                                    render_message_body(ui, &body_with_emoji);
                                    // Reactions display
                                    if let Some(msg_id) = message.id {
                                        let reactions = self.reactions.get(&msg_id).cloned().unwrap_or_default();
                                        if !reactions.is_empty() {
                                            ui.horizontal(|ui| {
                                                // Group by emoji
                                                let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
                                                for (emoji, _) in &reactions {
                                                    *counts.entry(emoji.clone()).or_default() += 1;
                                                }
                                                for (emoji, count) in counts {
                                                    ui.label(egui::RichText::new(format!("{} {}", emoji, count)).small());
                                                }
                                            });
                                        }
                                        // Reaction picker on hover
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 2.0;
                                            for emoji in ["👍","❤️","😂","😮","😢","🔥"] {
                                                if ui.small_button(emoji).clicked() {
                                                    let _ = self.network.tx.send(UiToNet::ReactToMessage {
                                                        message_id: msg_id,
                                                        emoji: emoji.to_string(),
                                                    });
                                                }
                                            }
                                        });
                                    }
                                    // Edit/delete context menu for own messages
                                    if Some(&message.from) == self.logged_in_user.as_ref() {
                                        if let Some(msg_id) = message.id {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 4.0;
                                                if ui.small_button("✏️").on_hover_text("Edit").clicked() {
                                                    self.editing_message = Some((msg_id, message.body.trim_end_matches(" ✏️").to_string()));
                                                }
                                                if ui.small_button("🗑").on_hover_text("Delete").clicked() {
                                                    let _ = self.network.tx.send(UiToNet::DeleteMessage { message_id: msg_id });
                                                }
                                                if ui.small_button("↩️").on_hover_text("Reply").clicked() {
                                                    let snippet = message.body.chars().take(50).collect::<String>();
                                                    self.replying_to = Some((msg_id, message.from.clone(), snippet));
                                                }
                                            });
                                        }
                                    } else {
                                        // Reply button for other users' messages
                                        if let Some(msg_id) = message.id {
                                            if ui.small_button("↩️").on_hover_text("Reply").clicked() {
                                                let snippet = message.body.chars().take(50).collect::<String>();
                                                self.replying_to = Some((msg_id, message.from.clone(), snippet));
                                            }
                                        }
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                        if self
                            .search_results
                            .get(&self.selected_target)
                            .map(|result| result.messages.is_empty())
                            .unwrap_or(true)
                            && using_search
                        {
                            ui.label("No search results yet.");
                        } else if self
                            .messages
                            .get(&self.selected_target)
                            .map(|list| list.is_empty())
                            .unwrap_or(true)
                            && !using_search
                        {
                            ui.label("Say hi to start a conversation.");
                        }
                    });
                    // Typing indicator
                    let typing_key = match &self.selected_target {
                        ChatTarget::Lobby => "lobby".to_string(),
                        ChatTarget::Room(id) => id.clone(),
                        ChatTarget::Direct(name) => name.clone(),
                    };
                    // Clear stale typing indicators (older than 5 seconds)
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let now = std::time::Instant::now();
                        let stale: Vec<String> = self.typing_timeout.iter()
                            .filter(|(_, t)| now.duration_since(**t).as_secs() >= 5)
                            .map(|(k, _)| k.clone())
                            .collect();
                        for key in stale {
                            if let Some((room, user)) = key.split_once(':') {
                                if let Some(users) = self.typing_users.get_mut(room) {
                                    users.retain(|u| u != user);
                                }
                                self.typing_timeout.remove(&key);
                            }
                        }
                    }
                    if let Some(typers) = self.typing_users.get(&typing_key) {
                        let typers: Vec<_> = typers.iter()
                            .filter(|u| Some(*u) != self.logged_in_user.as_ref())
                            .collect();
                        if !typers.is_empty() {
                            let text = if typers.len() == 1 {
                                format!("{} is typing...", typers[0])
                            } else {
                                format!("{} people are typing...", typers.len())
                            };
                            ui.label(egui::RichText::new(text).small().italics().color(egui::Color32::GRAY));
                        }
                    }
                    ui.add_space(4.0);

                    // Edit message modal
                    if self.editing_message.is_some() {
                        let (msg_id, mut edit_text) = self.editing_message.take().unwrap();
                        let mut done = false;
                        let mut save = false;
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Editing:").small().color(egui::Color32::YELLOW));
                            let edit_response = ui.add(
                                egui::TextEdit::singleline(&mut edit_text)
                                    .desired_width(f32::INFINITY)
                            );
                            save = ui.button("Save").clicked()
                                || (edit_response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                            done = save || ui.button("Cancel").clicked()
                                || ui.input(|i| i.key_pressed(egui::Key::Escape));
                        });
                        if save {
                            let _ = self.network.tx.send(UiToNet::EditMessage { message_id: msg_id, new_body: edit_text.clone() });
                        }
                        if !done {
                            self.editing_message = Some((msg_id, edit_text));
                        }
                    } else {
                    // Reply indicator
                    let reply_display = self.replying_to.as_ref().map(|(_, from, snippet)| (from.clone(), snippet.clone()));
                    if let Some((reply_from, reply_snippet)) = reply_display {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("↩️ Replying to {}: \"{}\"", reply_from, reply_snippet))
                                .small().color(egui::Color32::YELLOW));
                            if ui.small_button("✖").clicked() {
                                self.replying_to = None;
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.chat_input)
                                .hint_text("Type a message and press Enter to send")
                                .desired_width(f32::INFINITY),
                        );

                        // Send on Enter (no modifier needed), or clicking Send
                        let should_send = ui.button("Send").clicked()
                            || (!self.chat_input.is_empty()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)));

                        // Typing indicators — notify server when input changes
                        if response.has_focus() && response.changed() {
                            let room_key = match &self.selected_target {
                                ChatTarget::Room(id) => Some(id.clone()),
                                _ => None,
                            };
                            if let Some(room_id) = room_key {
                                if self.chat_input.is_empty() {
                                    let _ = self.network.tx.send(UiToNet::StopTyping { room_id });
                                } else {
                                    let _ = self.network.tx.send(UiToNet::StartTyping { room_id });
                                }
                            }
                        }

                        if should_send {
                            // Stop typing indicator when message is sent
                            if let ChatTarget::Room(room_id) = &self.selected_target.clone() {
                                let _ = self.network.tx.send(UiToNet::StopTyping { room_id: room_id.clone() });
                            }
                            
                            // Handle reply if set
                            let reply_info = self.replying_to.take();
                            if let Some((reply_id, _, _)) = reply_info {
                                let body = Self::sanitize_input(&self.chat_input);
                                if !body.is_empty() {
                                    match &self.selected_target {
                                        ChatTarget::Lobby => {
                                            let _ = self.network.tx.send(UiToNet::ReplyToMessage { reply_to_id: reply_id, body });
                                        }
                                        ChatTarget::Direct(to) => {
                                            let _ = self.network.tx.send(UiToNet::ReplyToDirect { 
                                                to: to.clone(), 
                                                reply_to_id: reply_id, 
                                                body 
                                            });
                                        }
                                        ChatTarget::Room(_) => {
                                            // Room replies not yet implemented on server
                                            self.send_chat();
                                        }
                                    }
                                    self.audio_manager.play(SoundEffect::MessageSent);
                                    self.chat_input.clear();
                                }
                            } else {
                                self.send_chat();
                            }
                            response.request_focus();
                        } else {
                            // Keep focus on input so Enter always works, but not if a modal is open
                            if !response.has_focus() && !self.show_add_friend_modal && !self.show_friend_requests_modal && self.viewing_profile.is_none() {
                                response.request_focus();
                            }
                        }

                        // Escape to clear input and cancel reply
                        if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.chat_input.clear();
                            self.replying_to = None;
                        }
                    }); // end horizontal input
                    } // end else (not editing)
                }); // end CentralPanel
            }
        }
    }
}

fn apply_theme(ctx: &egui::Context, theme: Theme) {
    let mut visuals = match theme {
        Theme::Light => {
            let mut visuals = egui::Visuals::light();
            visuals.panel_fill = egui::Color32::from_rgb(245, 240, 225);
            visuals.window_fill = egui::Color32::from_rgb(250, 245, 235);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(235, 230, 210);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(255, 250, 230);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(255, 221, 128);
            visuals.selection.bg_fill = egui::Color32::from_rgb(255, 199, 69);
            visuals.override_text_color = Some(egui::Color32::from_rgb(35, 30, 25));
            visuals
        }
        Theme::Dark => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(30, 28, 26);
            visuals.window_fill = egui::Color32::from_rgb(36, 34, 32);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(44, 42, 40);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 48, 46);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(86, 74, 52);
            visuals.selection.bg_fill = egui::Color32::from_rgb(198, 154, 69);
            visuals.override_text_color = Some(egui::Color32::from_rgb(235, 225, 210));
            visuals
        }
        Theme::MidnightAmber => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(30, 28, 26);
            visuals.window_fill = egui::Color32::from_rgb(35, 32, 28);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(42, 39, 34);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(48, 44, 38);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(108, 72, 30);
            visuals.selection.bg_fill = egui::Color32::from_rgb(240, 168, 58);
            visuals.override_text_color = Some(egui::Color32::from_rgb(231, 220, 198));
            visuals
        }
    };
    visuals.window_corner_radius = egui::CornerRadius::same(6);
    ctx.set_visuals(visuals);
}

fn format_relative_time(at: &str) -> String {
    let parsed = chrono::DateTime::parse_from_rfc3339(at).ok();
    let timestamp = match parsed {
        Some(value) => value.with_timezone(&chrono::Utc),
        None => return String::new(),
    };
    let now = chrono::Utc::now();
    let delta = now.signed_duration_since(timestamp);
    let secs = delta.num_seconds();
    if secs < 0 {
        return "now".to_string();
    }
    if secs < 60 {
        return "just now".to_string();
    }
    let mins = delta.num_minutes();
    if mins < 60 {
        return format!("{}m", mins);
    }
    let hours = delta.num_hours();
    if hours < 24 {
        return format!("{}h", hours);
    }
    let days = delta.num_days();
    if days < 7 {
        return format!("{}d", days);
    }
    format!("{}w", days / 7)
}

fn format_full_timestamp(at: &str) -> String {
    let parsed = chrono::DateTime::parse_from_rfc3339(at).ok();
    let timestamp = match parsed {
        Some(value) => value.with_timezone(&chrono::Local),
        None => return String::new(),
    };
    timestamp.format("%B %d, %Y at %I:%M %p").to_string()
}

fn format_idle_time(secs: i64) -> String {
    if secs < 60 {
        return format!("{}s", secs);
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{}m", mins);
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{}h", hours);
    }
    let days = hours / 24;
    format!("{}d", days)
}

// Render message body with clickable URLs
fn render_message_body(ui: &mut egui::Ui, text: &str) {
    // Split on whitespace, detect URLs, render as hyperlinks
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for word in text.split_whitespace() {
            if word.starts_with("http://") || word.starts_with("https://") {
                if ui.hyperlink(word).clicked() {
                    // egui handles opening the URL via hyperlink widget
                }
            } else {
                ui.label(word);
            }
        }
    });
}

// Convert classic AIM emoticons to emoji
fn convert_emoticons(text: &str) -> String {    text.replace(":)", "😊")
        .replace(":(", "😢")
        .replace(":D", "😄")
        .replace(";)", "😉")
        .replace(":P", "😛")
        .replace(":p", "😛")
        .replace("<3", "❤️")
        .replace(":|", "😐")
        .replace(":o", "😮")
        .replace(":O", "😮")
        .replace("8)", "😎")
        .replace(":*", "😘")
        .replace(":'(", "😢")
        .replace("XD", "😆")
        .replace("^_^", "😊")
        .replace("-_-", "😑")
}

// Generate a color from username (consistent colors per user)
fn username_to_color(username: &str) -> egui::Color32 {
    let mut hash: u32 = 0;
    for byte in username.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }

    // Generate pleasant, distinct colors (not too dark, not too light)
    let hue = (hash % 360) as f32;
    let saturation = 0.6 + ((hash >> 8) % 20) as f32 / 100.0; // 0.6-0.8
    let value = 0.7 + ((hash >> 16) % 20) as f32 / 100.0; // 0.7-0.9

    // HSV to RGB conversion
    let c = value * saturation;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = value - c;

    let (r, g, b) = if hue < 60.0 {
        (c, x, 0.0)
    } else if hue < 120.0 {
        (x, c, 0.0)
    } else if hue < 180.0 {
        (0.0, c, x)
    } else if hue < 240.0 {
        (0.0, x, c)
    } else if hue < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    egui::Color32::from_rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

// Get user initials (first 2 chars, uppercase)
fn get_initials(username: &str) -> String {
    username
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

// Native network implementation
#[cfg(not(target_arch = "wasm32"))]
fn spawn_network() -> NetworkHandle {
    let (ui_tx, ui_rx) = mpsc::unbounded_channel::<UiToNet>();
    let (net_tx, net_rx) = std_mpsc::channel::<NetToUi>();

    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("failed to create runtime");
        runtime.block_on(async move {
            network_task(ui_rx, net_tx).await;
        });
    });

    NetworkHandle { tx: ui_tx, rx: net_rx }
}

#[cfg(not(target_arch = "wasm32"))]
async fn network_task(
    mut ui_rx: mpsc::UnboundedReceiver<UiToNet>,
    net_tx: std_mpsc::Sender<NetToUi>,
) {
    while let Some(command) = ui_rx.recv().await {
        match command {
            UiToNet::Connect {
                url,
                username,
                password,
                mode,
            } => {
                let result = run_connection(url, username, password, mode, &mut ui_rx, &net_tx).await;
                if let Err(message) = result {
                    let _ = net_tx.send(NetToUi::Error(message));
                }
            }
            UiToNet::Disconnect => {
                let _ = net_tx.send(NetToUi::Disconnected);
            }
            UiToNet::AddFriend { username: _ } => {
                // Send AddFriend in a temporary connection (or extend run_connection to handle it if needed)
                // For now, send via a new connection if needed, or handle in run_connection if connected
                // This is handled in run_connection's select! branch
            }
            _ => {}
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_connection(
    url: String,
    username: String,
    password: String,
    mode: AuthMode,
    ui_rx: &mut mpsc::UnboundedReceiver<UiToNet>,
    net_tx: &std_mpsc::Sender<NetToUi>,
) -> Result<(), String> {
    eprintln!("[DEBUG] Connecting to: {}", url);

    // ws:// — plain TCP to avoid rustls interference
    // wss:// — connect_async with TLS
    if url.starts_with("ws://") {
        let host_port = url.trim_start_matches("ws://").split('/').next()
            .unwrap_or("localhost:9001");
        eprintln!("[DEBUG] TCP connecting to: {}", host_port);
        let tcp = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::net::TcpStream::connect(host_port),
        )
        .await
        .map_err(|_| "Connection timed out — is the server running?".to_string())?
        .map_err(|e| e.to_string())?;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut req = url.as_str().into_client_request().map_err(|e| e.to_string())?;
        req.headers_mut().insert("Host",
            host_port.parse().map_err(|e: tokio_tungstenite::tungstenite::http::header::InvalidHeaderValue| e.to_string())?);
        let (ws, _) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio_tungstenite::client_async(req, tcp),
        )
        .await
        .map_err(|_| "WebSocket handshake timed out".to_string())?
        .map_err(|e| e.to_string())?;
        eprintln!("[DEBUG] WebSocket connected!");
        run_ws(ws, username, password, mode, ui_rx, net_tx).await
    } else {
        eprintln!("[DEBUG] TLS connecting...");
        let (ws, _) = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            connect_async(&url),
        )
        .await
        .map_err(|_| "Connection timed out — is the server running?".to_string())?
        .map_err(|e| e.to_string())?;
        eprintln!("[DEBUG] TLS WebSocket connected!");
        run_ws(ws, username, password, mode, ui_rx, net_tx).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_ws<S>(
    ws: tokio_tungstenite::WebSocketStream<S>,
    username: String,
    password: String,
    mode: AuthMode,
    ui_rx: &mut mpsc::UnboundedReceiver<UiToNet>,
    net_tx: &std_mpsc::Sender<NetToUi>,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (mut ws_tx, mut ws_rx) = ws.split();

    match mode {
        AuthMode::Login => {
            send_json(&mut ws_tx, ClientToServer::Login { username, password }).await?;
        }
        AuthMode::Register => {
            send_json(&mut ws_tx, ClientToServer::Register { username, password }).await?;
        }
    }
    let _ = net_tx.send(NetToUi::Connected);

    loop {
        tokio::select! {
            incoming = ws_rx.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(event) = serde_json::from_str::<ServerToClient>(&text) {
                            match event {
                                ServerToClient::Welcome { message } => {
                                    let _ = net_tx.send(NetToUi::Chat { from: "Server".to_string(), body: message });
                                }
                                ServerToClient::AuthOk { username } => {
                                    let _ = net_tx.send(NetToUi::AuthOk { username });
                                }
                                ServerToClient::AuthError { message } => {
                                    let _ = net_tx.send(NetToUi::AuthError(message));
                                }
                                ServerToClient::Presence { users } => {
                                    let _ = net_tx.send(NetToUi::Presence(users));
                                }
                                ServerToClient::Chat { from, body } => {
                                    let _ = net_tx.send(NetToUi::Chat { from, body });
                                }
                                ServerToClient::DirectMessage { from, body } => {
                                    let _ = net_tx.send(NetToUi::DirectMessage { from, body });
                                }
                                ServerToClient::Threads { users } => {
                                    let _ = net_tx.send(NetToUi::Threads(users));
                                }
                                ServerToClient::History { target, messages } => {
                                    let mapped_target = match target {
                                        HistoryTarget::Lobby => ChatTarget::Lobby,
                                        HistoryTarget::Direct { username } => ChatTarget::Direct(username),
                                        HistoryTarget::Room { room_id } => ChatTarget::Room(room_id),
                                    };
                                    let mapped_messages = messages
                                        .into_iter()
                                        .map(|record| ChatMessage {
                                            from: record.from,
                                            body: record.body,
                                            at: record.at,
                                            id: record.id,
                                            read_count: record.read_count,
                                        })
                                        .collect::<Vec<_>>();
                                    let _ = net_tx.send(NetToUi::History {
                                        target: mapped_target,
                                        messages: mapped_messages,
                                    });
                                }
                                ServerToClient::SearchResults { target, query, messages } => {
                                    let mapped_target = match target {
                                        HistoryTarget::Lobby => ChatTarget::Lobby,
                                        HistoryTarget::Direct { username } => ChatTarget::Direct(username),
                                        HistoryTarget::Room { room_id } => ChatTarget::Room(room_id),
                                    };
                                    let mapped_messages = messages
                                        .into_iter()
                                        .map(|record| ChatMessage {
                                            from: record.from,
                                            body: record.body,
                                            at: record.at,
                                            id: record.id,
                                            read_count: record.read_count,
                                        })
                                        .collect::<Vec<_>>();
                                    let _ = net_tx.send(NetToUi::SearchResults {
                                        target: mapped_target,
                                        query,
                                        messages: mapped_messages,
                                    });
                                }
                                ServerToClient::System { message } => {
                                    let _ = net_tx.send(NetToUi::System(message));
                                }
                                ServerToClient::AddFriendResult { _username, success, message } => {
                                    let _ = net_tx.send(NetToUi::AddFriendResult { _username, success, message });
                                }
                                ServerToClient::FriendRequest { from } => {
                                    let _ = net_tx.send(NetToUi::FriendRequest { from });
                                }
                                ServerToClient::FriendRequestResult { username, accepted } => {
                                    let _ = net_tx.send(NetToUi::FriendRequestResult { username, accepted });
                                }
                                ServerToClient::KeyExchange { from, public_key } => {
                                    let _ = net_tx.send(NetToUi::KeyExchange { from, public_key });
                                }
                                ServerToClient::EncryptedDirectMessage { from, encrypted_body } => {
                                    let _ = net_tx.send(NetToUi::EncryptedDirectMessage { from, encrypted_body });
                                }
                                ServerToClient::ChatRoomCreated { room_id, name } => {
                                    let _ = net_tx.send(NetToUi::ChatRoomCreated { room_id, name });
                                }
                                ServerToClient::ChatRoomList { rooms } => {
                                    let rooms_vec = rooms.into_iter()
                                        .map(|r| (r.id, r.name, r.member_count))
                                        .collect();
                                    let _ = net_tx.send(NetToUi::ChatRoomList { rooms: rooms_vec });
                                }
                                ServerToClient::RoomMessage { room_id, from, body, message_id } => {
                                    let _ = net_tx.send(NetToUi::RoomMessage { room_id, from, body, message_id });
                                }
                                ServerToClient::UserJoinedRoom { room_id, username } => {
                                    let _ = net_tx.send(NetToUi::UserJoinedRoom { room_id, username });
                                }
                                ServerToClient::UserLeftRoom { room_id, username } => {
                                    let _ = net_tx.send(NetToUi::UserLeftRoom { room_id, username });
                                }
                                ServerToClient::RoomMembers { room_id, members } => {
                                    let _ = net_tx.send(NetToUi::RoomMembers { room_id, members });
                                }
                                ServerToClient::UserTyping { room_id, username } => {
                                    let _ = net_tx.send(NetToUi::UserTyping { room_id, username });
                                }
                                ServerToClient::UserStoppedTyping { room_id, username } => {
                                    let _ = net_tx.send(NetToUi::UserStoppedTyping { room_id, username });
                                }
                                ServerToClient::ReadReceipt { message_id, read_by } => {
                                    let _ = net_tx.send(NetToUi::ReadReceipt { message_id, read_by });
                                }
                                ServerToClient::MessageEdited { message_id, new_body, edited_by: _ } => {
                                    let _ = net_tx.send(NetToUi::MessageEdited { message_id, new_body });
                                }
                                ServerToClient::MessageDeleted { message_id, deleted_by: _ } => {
                                    let _ = net_tx.send(NetToUi::MessageDeleted { message_id });
                                }
                                ServerToClient::MessageReaction { message_id, emoji, from } => {
                                    let _ = net_tx.send(NetToUi::MessageReaction { message_id, emoji, from });
                                }
                                ServerToClient::Nudged { from } => {
                                    let _ = net_tx.send(NetToUi::Nudged { from });
                                }
                                ServerToClient::Winked { from, emoji } => {
                                    let _ = net_tx.send(NetToUi::Winked { from, emoji });
                                }
                                ServerToClient::ProfileData { username, bio, status, joined, avatar_url } => {
                                    let _ = net_tx.send(NetToUi::ProfileData { username, bio, status, joined, avatar_url });
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                    None => break,
                }
            }
            Some(command) = ui_rx.recv() => {
                match command {
                    UiToNet::SendChat { body } => {
                        send_json(&mut ws_tx, ClientToServer::Chat { body }).await?;
                    }
                    UiToNet::SendDirect { to, body } => {
                        send_json(&mut ws_tx, ClientToServer::DirectMessage { to, body }).await?;
                    }
                    UiToNet::ExchangeKey { to, public_key } => {
                        send_json(&mut ws_tx, ClientToServer::ExchangeKey { to, public_key }).await?;
                    }
                    UiToNet::SendEncryptedDirect { to, encrypted_body } => {
                        send_json(&mut ws_tx, ClientToServer::EncryptedDirectMessage { to, encrypted_body }).await?;
                    }
                    UiToNet::FetchHistory { target } => {
                        let target = match target {
                            ChatTarget::Lobby => HistoryTarget::Lobby,
                            ChatTarget::Direct(username) => HistoryTarget::Direct { username },
                            ChatTarget::Room(room_id) => HistoryTarget::Room { room_id },
                        };
                        send_json(&mut ws_tx, ClientToServer::FetchHistory { target }).await?;
                    }
                    UiToNet::FetchThreads => {
                        send_json(&mut ws_tx, ClientToServer::FetchThreads).await?;
                    }
                    UiToNet::Search { target, query } => {
                        let target = match target {
                            ChatTarget::Lobby => HistoryTarget::Lobby,
                            ChatTarget::Direct(username) => HistoryTarget::Direct { username },
                            ChatTarget::Room(room_id) => HistoryTarget::Room { room_id },
                        };
                        send_json(&mut ws_tx, ClientToServer::Search { target, query }).await?;
                    }
                    UiToNet::SetAway { away } => {
                        send_json(&mut ws_tx, ClientToServer::SetAway { away }).await?;
                    }
                    UiToNet::Block { username } => {
                        send_json(&mut ws_tx, ClientToServer::Block { username }).await?;
                    }
                    UiToNet::Unblock { username } => {
                        send_json(&mut ws_tx, ClientToServer::Unblock { username }).await?;
                    }
                    UiToNet::Mute { username } => {
                        send_json(&mut ws_tx, ClientToServer::Mute { username }).await?;
                    }
                    UiToNet::Unmute { username } => {
                        send_json(&mut ws_tx, ClientToServer::Unmute { username }).await?;
                    }
                    UiToNet::Report { username, reason } => {
                        send_json(&mut ws_tx, ClientToServer::Report { username, reason }).await?;
                    }
                    UiToNet::AddFriend { username } => {
                        send_json(&mut ws_tx, ClientToServer::AddFriend { username }).await?;
                    }
                    UiToNet::AcceptFriendRequest { username } => {
                        send_json(&mut ws_tx, ClientToServer::AcceptFriendRequest { username }).await?;
                    }
                    UiToNet::DeclineFriendRequest { username } => {
                        send_json(&mut ws_tx, ClientToServer::DeclineFriendRequest { username }).await?;
                    }
                    UiToNet::CreateChatRoom { name } => {
                        send_json(&mut ws_tx, ClientToServer::CreateChatRoom { name }).await?;
                    }
                    UiToNet::JoinChatRoom { room_id } => {
                        send_json(&mut ws_tx, ClientToServer::JoinChatRoom { room_id }).await?;
                    }
                    UiToNet::LeaveChatRoom { room_id } => {
                        send_json(&mut ws_tx, ClientToServer::LeaveChatRoom { room_id }).await?;
                    }
                    UiToNet::SendRoomMessage { room_id, body } => {
                        send_json(&mut ws_tx, ClientToServer::SendRoomMessage { room_id, body }).await?;
                    }
                    UiToNet::FetchChatRooms => {
                        send_json(&mut ws_tx, ClientToServer::FetchChatRooms).await?;
                    }
                    UiToNet::FetchRoomMembers { room_id } => {
                        send_json(&mut ws_tx, ClientToServer::FetchRoomMembers { room_id }).await?;
                    }
                    UiToNet::StartTyping { room_id } => {
                        send_json(&mut ws_tx, ClientToServer::StartTyping { room_id }).await?;
                    }
                    UiToNet::StopTyping { room_id } => {
                        send_json(&mut ws_tx, ClientToServer::StopTyping { room_id }).await?;
                    }
                    UiToNet::MarkMessageAsRead { message_id } => {
                        send_json(&mut ws_tx, ClientToServer::MarkMessageAsRead { message_id }).await?;
                    }
                    UiToNet::EditMessage { message_id, new_body } => {
                        send_json(&mut ws_tx, ClientToServer::EditMessage { message_id, new_body }).await?;
                    }
                    UiToNet::DeleteMessage { message_id } => {
                        send_json(&mut ws_tx, ClientToServer::DeleteMessage { message_id }).await?;
                    }
                    UiToNet::ReactToMessage { message_id, emoji } => {
                        send_json(&mut ws_tx, ClientToServer::ReactToMessage { message_id, emoji }).await?;
                    }
                    UiToNet::Nudge { to } => {
                        send_json(&mut ws_tx, ClientToServer::Nudge { to }).await?;
                    }
                    UiToNet::Wink { to, emoji } => {
                        send_json(&mut ws_tx, ClientToServer::Wink { to, emoji }).await?;
                    }
                    UiToNet::SetStatus { status } => {
                        send_json(&mut ws_tx, ClientToServer::SetStatus { status }).await?;
                    }
                    UiToNet::SetBio { bio } => {
                        send_json(&mut ws_tx, ClientToServer::SetBio { bio }).await?;
                    }
                    UiToNet::FetchProfile { username } => {
                        send_json(&mut ws_tx, ClientToServer::FetchProfile { username }).await?;
                    }
                    UiToNet::ReplyToMessage { reply_to_id, body } => {
                        send_json(&mut ws_tx, ClientToServer::ReplyToMessage { reply_to_id, body }).await?;
                    }
                    UiToNet::ReplyToDirect { to, reply_to_id, body } => {
                        send_json(&mut ws_tx, ClientToServer::ReplyToDirect { to, reply_to_id, body }).await?;
                    }
                    UiToNet::SetAvatar { avatar_data } => {
                        send_json(&mut ws_tx, ClientToServer::SetAvatar { avatar_data }).await?;
                    }
                    UiToNet::Disconnect => {
                        let _ = ws_tx.send(Message::Close(None)).await;
                        break;
                    }
                    UiToNet::Connect { .. } => {}
                }
            }
        }
    }

    let _ = net_tx.send(NetToUi::Disconnected);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
async fn send_json<S>(ws_tx: &mut S, payload: ClientToServer) -> Result<(), String>
where
    S: Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    let text = serde_json::to_string(&payload).map_err(|err| err.to_string())?;
    ws_tx
        .send(Message::Text(text))
        .await
        .map_err(|err| err.to_string())
}

// Web network stub
#[cfg(target_arch = "wasm32")]
fn spawn_network() -> NetworkHandle {
    let (ui_tx, _ui_rx) = mpsc::unbounded_channel::<UiToNet>();
    let (net_tx, net_rx) = std_mpsc::channel::<NetToUi>();

    // On web, we'll handle login differently - just return a handle
    // In the UI, we'll detect this is web and handle networking specially
    
    NetworkHandle { tx: ui_tx, rx: net_rx }
}


// Native entry point
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(920.0, 640.0)),
        ..Default::default()
    };
    eframe::run_native(
        "AOL-Style Messenger",
        options,
        Box::new(|cc| {
            let mut app = AolApp::new(cc);
            app.load_credentials();
            Ok(Box::new(app))
        }),
    )
}

// Web entry point
#[cfg(target_arch = "wasm32")]
fn main() {
    // Initialize panic hook for better error messages in the browser console
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log (optional)
    // tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id",
                web_options,
                Box::new(|cc| Ok(Box::new(AolApp::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}

#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;
use std::time::Instant;
use std::thread;

use chrono::Utc;
use eframe::egui;
use futures_util::{Sink, SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use chatmessagediscordclone::protocol::{
    ClientToServer, HistoryTarget, ServerToClient, UserStatus, UserInfo,
};
use chatmessagediscordclone::db::LocalDb;
use chatmessagediscordclone::update;
use chatmessagediscordclone::{ChatMessage, ChatTarget};

struct Toast {
    text: String,
    kind: ToastKind,
    ttl: f32,
}

impl Toast {
    fn new(text: String) -> Self {
        Self {
            text,
            kind: ToastKind::Info,
            ttl: 3.0,
        }
    }
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

enum UiToNet {
    Connect {
        url: String,
        username: String,
        password: String,
        first_name: String,
        last_name: String,
        mode: AuthMode,
    },
    SendChat { body: String },
    SendDirect { to: String, body: String },
    FetchHistory { target: ChatTarget },
    FetchThreads,
    Search { target: ChatTarget, query: String },
    SetAway { away: Option<String> },
    Block { username: String },
    Unblock { username: String },
    Mute { username: String },
    Unmute { username: String },
    Report { username: String, reason: String },
    AddFriend { username: String, nickname: Option<String> },
    FriendRequest { to: String },
    RespondToFriendRequest { from: String, accepted: bool },
    GetUsers,
    Disconnect,
}

enum NetToUi {
    Connected,
    Disconnected,
    Presence(Vec<UserStatus>),
    Chat { from: String, body: String },
    DirectMessage { from: String, body: String },
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
    FriendAdded { username: String },
    FriendRequest { from: String },
    FriendRequestResponse { from: String, accepted: bool },
    Users { users: Vec<UserInfo> },
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
    update_rx: std_mpsc::Receiver<String>,
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
    away_text: String,
    chat_input: String,
    dm_target: String,
    selected_target: ChatTarget,
    report_reason: String,
    logging_in: bool,
    login_started_at: Option<Instant>,
    search_query: String,
    search_in_progress: bool,
    search_results: HashMap<ChatTarget, SearchResult>,
    buddies: Vec<UserStatus>,
    recent_threads: Vec<String>,
    messages: std::collections::HashMap<ChatTarget, Vec<ChatMessage>>,
    toast: Option<Toast>,
    db: Option<LocalDb>,
    offline_queue_size: usize,
    update_available: Option<String>,
    checked_updates: bool,
    updating: bool,
    friends: Vec<(String, Option<String>)>,
    add_friend_username: String,
    add_friend_nickname: String,
    pending_friend_requests: Vec<String>,
    first_name: String,
    last_name: String,
    registered_users: Vec<UserInfo>,
}

struct SearchResult {
    query: String,
    messages: Vec<ChatMessage>,
}

impl AolApp {
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

        let db = LocalDb::new().ok();

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
            server_url: "wss://blast-from-the-past-messenger.fly.dev".to_string(),
            username: "RetroUser".to_string(),
            password: String::new(),
            confirm_password: String::new(),
            show_password: false,
            show_confirm_password: false,
            away_text: String::new(),
            chat_input: String::new(),
            dm_target: String::new(),
            selected_target: ChatTarget::Lobby,
            report_reason: String::new(),
            logging_in: false,
            login_started_at: None,
            search_query: String::new(),
            search_in_progress: false,
            search_results: HashMap::new(),
            buddies: Vec::new(),
            recent_threads: Vec::new(),
            messages: HashMap::new(),
            toast: None,
            db,
            offline_queue_size: 0,
            update_available: None,
            checked_updates: false,
            updating: false,
            friends: Vec::new(),
            add_friend_username: String::new(),
            add_friend_nickname: String::new(),
            pending_friend_requests: Vec::new(),
            first_name: String::new(),
            last_name: String::new(),
            registered_users: Vec::new(),
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
                    self.sync_queued_messages();
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
                    self.buddies = users;
                }
                NetToUi::Chat { from, body } => {
                    let timestamp = Utc::now().to_rfc3339();
                    let entry = self.messages.entry(ChatTarget::Lobby).or_default();
                    entry.push(ChatMessage {
                        from: from.clone(),
                        body: body.clone(),
                        at: timestamp.clone(),
                    });
                    if let Some(db) = &self.db {
                        let _ = db.save_message(&ChatTarget::Lobby, from, body, timestamp);
                    }
                }
                NetToUi::DirectMessage { from, body } => {
                    let target = ChatTarget::Direct(from.clone());
                    let timestamp = Utc::now().to_rfc3339();
                    let entry = self.messages.entry(target.clone()).or_default();
                    entry.push(ChatMessage {
                        from: from.clone(),
                        body: body.clone(),
                        at: timestamp.clone(),
                    });
                    if let Some(db) = &self.db {
                        let _ = db.save_message(&target, from, body, timestamp);
                    }
                }
                NetToUi::Threads(users) => {
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
                NetToUi::AuthOk { username } => {
                    self.logged_in_user = Some(username);
                    self.status = match self.auth_mode {
                        AuthMode::Register => "Account created. Logged in.".to_string(),
                        AuthMode::Login => "Logged in.".to_string(),
                    };
                    self.show_toast(self.status.clone(), ToastKind::Success);
                    self.screen = Screen::Chat;
                    self.auth_mode = AuthMode::Login;
                    self.confirm_password.clear();
                    self.password.clear();
                    self.logging_in = false;
                    self.login_started_at = None;
                    let _ = self
                        .network
                        .tx
                        .send(UiToNet::FetchHistory {
                            target: ChatTarget::Lobby,
                        });
                    let _ = self.network.tx.send(UiToNet::FetchThreads);
                    self.update_offline_queue_size();
                    if self.offline_queue_size > 0 {
                        self.show_toast(
                            format!("{} messages ready to sync", self.offline_queue_size),
                            ToastKind::Info,
                        );
                    }
                    // Load friends from database
                    if let Some(db) = &self.db {
                        if let Ok(friends) = db.get_friends() {
                            self.friends = friends;
                        }
                    }
                }
                NetToUi::AuthError(message) => {
                    self.status = message;
                    self.show_toast(self.status.clone(), ToastKind::Error);
                    self.logging_in = false;
                    self.login_started_at = None;
                }
                NetToUi::System(message) => {
                    let entry = self.messages.entry(ChatTarget::Lobby).or_default();
                    entry.push(ChatMessage {
                        from: "System".to_string(),
                        body: message,
                        at: Utc::now().to_rfc3339(),
                    });
                    self.show_toast("System message received".to_string(), ToastKind::Info);
                }
                NetToUi::Error(message) => {
                    self.status = format!("Error: {message}");
                    self.show_toast(self.status.clone(), ToastKind::Error);
                    self.logging_in = false;
                    self.login_started_at = None;
                }
                NetToUi::FriendAdded { username } => {
                    self.show_toast(format!("Added {} as friend!", username), ToastKind::Success);
                }
                NetToUi::FriendRequest { from } => {
                    if !self.pending_friend_requests.contains(&from) {
                        self.pending_friend_requests.push(from.clone());
                    }
                    self.show_toast(format!("{} sent you a friend request!", from), ToastKind::Info);
                }
                NetToUi::FriendRequestResponse { from, accepted } => {
                    if accepted {
                        self.show_toast(format!("{} accepted your friend request!", from), ToastKind::Success);
                    } else {
                        self.show_toast(format!("{} declined your friend request.", from), ToastKind::Info);
                    }
                }
                NetToUi::Users { users } => {
                    self.registered_users = users;
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
        if self.auth_mode == AuthMode::Register {
            if self.first_name.trim().is_empty() || self.last_name.trim().is_empty() {
                self.status = "First name and last name are required".to_string();
                self.show_toast(self.status.clone(), ToastKind::Error);
                return;
            }
        }
        self.logging_in = true;
        self.login_started_at = Some(Instant::now());
        self.status = format!("Logging in as {}...", self.username.trim());
        let mut url = self.server_url.trim().to_string();
        if let Some(stripped) = url.strip_prefix("https://") {
            url = format!("wss://{stripped}");
        } else if let Some(stripped) = url.strip_prefix("http://") {
            url = format!("ws://{stripped}");
        } else if !url.starts_with("ws://") && !url.starts_with("wss://") {
            url = format!("wss://{url}");
        }
        self.server_url = url.clone();
        let _ = self.network.tx.send(UiToNet::Connect {
            url,
            username: self.username.trim().to_string(),
            password: self.password.clone(),
            first_name: self.first_name.trim().to_string(),
            last_name: self.last_name.trim().to_string(),
            mode: self.auth_mode,
        });
    }

    fn send_chat(&mut self) {
        let body = self.chat_input.trim();
        if body.is_empty() {
            return;
        }
        match &self.selected_target {
            ChatTarget::Lobby => {
                if self.connected {
                    let _ = self
                        .network
                        .tx
                        .send(UiToNet::SendChat { body: body.to_string() });
                } else {
                    // Queue message for offline delivery
                    if let Some(db) = &self.db {
                        let _ = db.queue_message(&ChatTarget::Lobby, body.to_string());
                        self.update_offline_queue_size();
                        self.show_toast("Message queued (offline)".to_string(), ToastKind::Info);
                    }
                }
            }
            ChatTarget::Direct(target) => {
                let msg = ChatMessage {
                    from: self
                        .logged_in_user
                        .clone()
                        .unwrap_or_else(|| "Me".to_string()),
                    body: body.to_string(),
                    at: Utc::now().to_rfc3339(),
                };
                let entry = self
                    .messages
                    .entry(ChatTarget::Direct(target.clone()))
                    .or_default();
                entry.push(msg);

                if self.connected {
                    let _ = self.network.tx.send(UiToNet::SendDirect {
                        to: target.clone(),
                        body: body.to_string(),
                    });
                } else {
                    // Queue message for offline delivery
                    if let Some(db) = &self.db {
                        let _ = db.queue_message(&ChatTarget::Direct(target.clone()), body.to_string());
                        self.update_offline_queue_size();
                        self.show_toast("Message queued (offline)".to_string(), ToastKind::Info);
                    }
                }
            }
        }
        self.chat_input.clear();
    }

    fn send_away(&mut self) {
        let away = if self.away_text.trim().is_empty() {
            None
        } else {
            Some(self.away_text.trim().to_string())
        };
        let _ = self.network.tx.send(UiToNet::SetAway { away });
    }

    fn sync_queued_messages(&mut self) {
        if let Some(db) = &self.db {
            if let Ok(queued) = db.get_queued_messages() {
                let queued_count = queued.len();
                for (msg_id, target, body, _) in queued {
                    match &target {
                        ChatTarget::Lobby => {
                            let _ = self.network.tx.send(UiToNet::SendChat {
                                body: body.clone(),
                            });
                        }
                        ChatTarget::Direct(to) => {
                            let _ = self.network.tx.send(UiToNet::SendDirect {
                                to: to.clone(),
                                body: body.clone(),
                            });
                        }
                    }
                    let _ = db.mark_queued_sent(msg_id);
                }
                self.update_offline_queue_size();
                if queued_count > 0 {
                    self.show_toast(
                        format!("Synced {} offline messages", queued_count),
                        ToastKind::Info,
                    );
                }
            }
        }
    }

    fn update_offline_queue_size(&mut self) {
        if let Some(db) = &self.db {
            if let Ok(queued) = db.get_queued_messages() {
                self.offline_queue_size = queued.len();
            }
        }
    }

    fn send_moderation(&mut self, action: UiToNet) {
        let _ = self.network.tx.send(action);
    }

    fn select_target(&mut self, target: ChatTarget) {
        if self.selected_target == target {
            return;
        }
        self.selected_target = target.clone();
        self.search_query.clear();
        self.search_in_progress = false;
        self.search_results.remove(&target);
        
        // Try to load from local cache first
        if let Some(db) = &self.db {
            if let Ok(cached_messages) = db.get_messages(&target) {
                if !cached_messages.is_empty() {
                    self.messages.insert(target.clone(), cached_messages);
                }
            }
        }
        
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
                let frame = egui::Frame::none()
                    .fill(fill)
                    .stroke(stroke)
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0));
                frame.show(ui, |ui| {
                    ui.label(toast.text.clone());
                });
            });
    }
}

impl eframe::App for AolApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_theme(ctx, self.theme);
        if self.startup_repaint_left > 0 {
            self.startup_repaint_left = self.startup_repaint_left.saturating_sub(1);
            ctx.request_repaint();
        } else if !self.checked_updates {
            self.checked_updates = true;
        }
        
        // Check for update availability
        if let Ok(version) = self.network.update_rx.try_recv() {
            self.update_available = Some(version);
            self.show_toast("Update available! Click Update Now to download and install.".to_string(), ToastKind::Info);
        }
        
        self.process_net_events();
        if self.show_background {
            self.draw_background(ctx);
        }
        self.draw_toast(ctx);

        match self.screen {
            Screen::SignIn => {
                let (card_fill, card_stroke, text_color) = match self.theme {
                    Theme::Light => (
                        egui::Color32::from_rgb(255, 250, 235),
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(210, 200, 175)),
                        egui::Color32::from_rgb(35, 30, 25),
                    ),
                    Theme::Dark => (
                        egui::Color32::from_rgb(40, 38, 36),
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 85, 80)),
                        egui::Color32::from_rgb(235, 225, 210),
                    ),
                    Theme::MidnightAmber => (
                        egui::Color32::from_rgb(36, 33, 28),
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(92, 70, 48)),
                        egui::Color32::from_rgb(231, 220, 198),
                    ),
                };
                egui::TopBottomPanel::top("signin_top").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(text_color, "AOL-Style Messenger");
                        ui.separator();
                        let bg_label = if self.show_background { "BG: On" } else { "BG: Off" };
                        if ui.button(bg_label).clicked() {
                            self.show_background = !self.show_background;
                        }
                        let label = match self.theme {
                            Theme::Light => "Dark Mode",
                            Theme::Dark => "Midnight Amber",
                            Theme::MidnightAmber => "Light Mode",
                        };
                        if ui.button("Refresh UI").clicked() {
                            ctx.request_repaint();
                        }
                        if ui.button(label).clicked() {
                            self.toggle_theme(ctx);
                        }
                    });
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(24.0);
                        egui::Frame::none()
                            .fill(card_fill)
                            .stroke(card_stroke)
                            .rounding(egui::Rounding::same(10.0))
                            .inner_margin(egui::Margin::same(18.0))
                            .show(ui, |ui| {
                                ui.set_max_width(360.0);
                                ui.colored_label(text_color, "Sign in to your retro inbox");
                                ui.add_space(14.0);
                                ui.horizontal(|ui| {
                                    if ui
                                        .selectable_label(self.auth_mode == AuthMode::Login, "Login")
                                        .clicked()
                                    {
                                        self.auth_mode = AuthMode::Login;
                                    }
                                    if ui
                                        .selectable_label(self.auth_mode == AuthMode::Register, "Create account")
                                        .clicked()
                                    {
                                        self.auth_mode = AuthMode::Register;
                                    }
                                });
                                ui.add_space(10.0);
                                ui.add(egui::TextEdit::singleline(&mut self.username).hint_text("Screen name"));
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.password)
                                            .password(!self.show_password)
                                            .hint_text("Password"),
                                    );
                                    ui.checkbox(&mut self.show_password, "Show");
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
                                    ui.add(egui::TextEdit::singleline(&mut self.first_name).hint_text("First name"));
                                    ui.add(egui::TextEdit::singleline(&mut self.last_name).hint_text("Last name"));
                                }
                                ui.add(egui::TextEdit::singleline(&mut self.server_url).hint_text("Server URL"));
                                ui.add_space(12.0);
                                let button_label = match self.auth_mode {
                                    AuthMode::Login => "Sign On",
                                    AuthMode::Register => "Create Account",
                                };
                                if ui.button(button_label).clicked() {
                                    self.send_connect();
                                }
                                ui.add_space(10.0);
                                if self.logging_in {
                                    let frames = ["[LOCKED]", "[LOCK--]", "[LOCK> ]", "[UNLOCK]"];
                                    let frame = if let Some(start) = self.login_started_at {
                                        let idx = ((start.elapsed().as_millis() / 200) % frames.len() as u128) as usize;
                                        frames[idx]
                                    } else {
                                        frames[0]
                                    };
                                    ui.horizontal(|ui| {
                                        ui.add(egui::Spinner::new());
                                        ui.label(format!("{frame} Logging in as {}...", self.username.trim()));
                                    });
                                }
                                ui.colored_label(text_color, format!("Status: {}", self.status));
                            });
                    });
                });
            }
            Screen::Chat => {
                egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("AOL Messenger");
                        ui.separator();
                        ui.label(format!("Status: {}", self.status));
                        if self.offline_queue_size > 0 {
                            ui.colored_label(
                                egui::Color32::YELLOW,
                                format!("⚠ {} queued", self.offline_queue_size),
                            );
                        }
                        if let Some(version) = &self.update_available {
                            ui.colored_label(
                                egui::Color32::LIGHT_BLUE,
                                format!("📦 Update: v{}", version),
                            );
                        }
                        if let Some(name) = &self.logged_in_user {
                            ui.label(format!("User: {name}"));
                        }
                        let bg_label = if self.show_background { "BG: On" } else { "BG: Off" };
                        if ui.button(bg_label).clicked() {
                            self.show_background = !self.show_background;
                        }
                        let label = match self.theme {
                            Theme::Light => "Dark Mode",
                            Theme::Dark => "Midnight Amber",
                            Theme::MidnightAmber => "Light Mode",
                        };
                        if ui.button("Refresh UI").clicked() {
                            ctx.request_repaint();
                        }
                        if ui.button(label).clicked() {
                            self.toggle_theme(ctx);
                        }
                        if ui.button("Disconnect").clicked() {
                            let _ = self.network.tx.send(UiToNet::Disconnect);
                            self.screen = Screen::SignIn;
                            self.logged_in_user = None;
                            self.selected_target = ChatTarget::Lobby;
                        }
                        if ui.button("👥 User List").clicked() {
                            let _ = self.network.tx.send(UiToNet::GetUsers);
                        }
                        if let Some(version) = &self.update_available {
                            if !self.updating {
                                if ui.button(format!("⬇ Update to {}", version)).clicked() {
                                    self.updating = true;
                                    let version = version.clone();
                                    thread::spawn(move || {
                                        let _ = download_and_install_update(&version);
                                    });
                                }
                            } else {
                                ui.label("Downloading update...");
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("💬");
                        ui.add(egui::TextEdit::singleline(&mut self.away_text).hint_text("Away...").desired_width(150.0));
                        if ui.small_button("Update").clicked() {
                            self.send_away();
                        }
                        ui.separator();
                        if self.offline_queue_size > 0 && ui.small_button(format!("⬆ Sync ({0})", self.offline_queue_size)).clicked() {
                            self.sync_queued_messages();
                        }
                        if let Some(version) = &self.update_available {
                            if ui.small_button(format!("⬇ Update to {}", version)).clicked() {
                                if let Err(e) = download_and_install_update(version) {
                                    self.toast = Some(Toast::new(format!("Update failed: {}", e)));
                                }
                            }
                        }
                    });
                });

                egui::SidePanel::left("buddy_list")
                    .resizable(false)
                    .min_width(250.0)
                    .show(ctx, |ui| {
                        ui.heading("Buddy List");
                        ui.separator();
                        
                        // Lobby button
                        if ui.selectable_label(self.selected_target == ChatTarget::Lobby, "📍 Lobby").clicked() {
                            self.select_target(ChatTarget::Lobby);
                        }
                        
                        // Recent DMs section
                        ui.separator();
                        egui::CollapsingHeader::new("📝 Recent DMs")
                            .default_open(true)
                            .show(ui, |ui| {
                                if self.recent_threads.is_empty() {
                                    ui.label("  (none)");
                                } else {
                                    for name in self.recent_threads.clone() {
                                        let target = ChatTarget::Direct(name.clone());
                                        if ui.selectable_label(self.selected_target == target, format!("  {0}", name)).clicked() {
                                            self.select_target(ChatTarget::Direct(name));
                                        }
                                    }
                                }
                            });
                        
                        // Open Direct Message
                        ui.separator();
                        egui::CollapsingHeader::new("✉️ Open Message")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.dm_target)
                                            .hint_text("Screen name")
                                            .desired_width(150.0),
                                    );
                                    if ui.small_button("Go").clicked() {
                                        let target = self.dm_target.trim();
                                        if !target.is_empty() {
                                            self.select_target(ChatTarget::Direct(target.to_string()));
                                            self.dm_target.clear();
                                        }
                                    }
                                });
                            });
                        
                        // Add Friend section
                        ui.separator();
                        egui::CollapsingHeader::new("➕ Add Friend")
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.add_friend_username)
                                        .hint_text("Username")
                                        .desired_width(200.0),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.add_friend_nickname)
                                        .hint_text("Nickname (optional)")
                                        .desired_width(200.0),
                                );
                                if ui.button("Add Friend").clicked() {
                                    let username = self.add_friend_username.trim().to_string();
                                    if !username.is_empty() {
                                        let nickname = if self.add_friend_nickname.trim().is_empty() {
                                            None
                                        } else {
                                            Some(self.add_friend_nickname.trim().to_string())
                                        };
                                        let _ = self.network.tx.send(UiToNet::AddFriend {
                                            username: username.clone(),
                                            nickname: nickname.clone(),
                                        });
                                        if let Some(db) = &self.db {
                                            let _ = db.add_friend(username.clone(), nickname.clone());
                                        }
                                        self.friends.push((username.clone(), nickname));
                                        self.add_friend_username.clear();
                                        self.add_friend_nickname.clear();
                                        self.toast = Some(Toast::new(format!("Added {0}", username)));
                                    }
                                }
                            });
                        
                        // Friends list
                        ui.separator();
                        egui::CollapsingHeader::new(format!("👥 Friends ({})", self.friends.len()))
                            .default_open(true)
                            .show(ui, |ui| {
                                if self.friends.is_empty() {
                                    ui.label("  (none)");
                                } else {
                                    for (friend, nickname) in self.friends.clone() {
                                        let display_name = nickname.as_ref().unwrap_or(&friend);
                                        ui.label(format!("  👤 {0}", display_name));
                                    }
                                }
                            });
                        
                        // Friend Requests section
                        ui.separator();
                        if !self.pending_friend_requests.is_empty() {
                            egui::CollapsingHeader::new(format!("🔔 Requests ({})", self.pending_friend_requests.len()))
                                .default_open(true)
                                .show(ui, |ui| {
                                    let current_user = self.logged_in_user.clone().unwrap_or_default();
                                    let mut to_accept = None;
                                    let mut to_decline = None;
                                    let pending_requests = self.pending_friend_requests.clone();
                                    for (idx, requester) in pending_requests.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("  from {0}", requester));
                                            if ui.small_button("✓").clicked() {
                                                to_accept = Some(idx);
                                            }
                                            if ui.small_button("✗").clicked() {
                                                to_decline = Some(idx);
                                            }
                                        });
                                    }
                                    
                                    // Process accept/decline after loop
                                    if let Some(idx) = to_accept {
                                        let requester = &self.pending_friend_requests[idx];
                                        let _ = self.network.tx.send(UiToNet::RespondToFriendRequest {
                                            from: requester.clone(),
                                            accepted: true,
                                        });
                                        if let Some(db) = &self.db {
                                            let _ = db.respond_to_friend_request(requester, &current_user, true);
                                        }
                                        self.friends.push((requester.clone(), None));
                                        self.pending_friend_requests.remove(idx);
                                    }
                                    if let Some(idx) = to_decline {
                                        let requester = &self.pending_friend_requests[idx];
                                        let _ = self.network.tx.send(UiToNet::RespondToFriendRequest {
                                            from: requester.clone(),
                                            accepted: false,
                                        });
                                        if let Some(db) = &self.db {
                                            let _ = db.respond_to_friend_request(requester, &current_user, false);
                                        }
                                        self.pending_friend_requests.remove(idx);
                                    }
                                });
                        }
                        
                        // Buddies Online section
                        ui.separator();
                        egui::CollapsingHeader::new(format!("🟢 Online ({})", self.buddies.len()))
                            .default_open(true)
                            .show(ui, |ui| {
                                if self.buddies.is_empty() {
                                    ui.label("  (none)");
                                } else {
                                    for buddy in self.buddies.clone() {
                                        let status = buddy
                                            .away
                                            .as_ref()
                                            .map(|_msg| format!("(Away)"))
                                            .unwrap_or_else(|| "".to_string());
                                        let label = if status.is_empty() {
                                            format!("  {0}", buddy.username)
                                        } else {
                                            format!("  {} {}", buddy.username, status)
                                        };
                                        let target = ChatTarget::Direct(buddy.username.clone());
                                        if ui.selectable_label(self.selected_target == target, label).clicked() {
                                            self.select_target(ChatTarget::Direct(buddy.username.clone()));
                                        }
                                    }
                                }
                            });
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    if !self.registered_users.is_empty() {
                        // Show admin dashboard with users
                        ui.heading("👥 Registered Users");
                        ui.horizontal(|ui| {
                            if ui.button("Close").clicked() {
                                self.registered_users.clear();
                            }
                        });
                        ui.separator();
                        
                        // Create table headers
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("Username").strong().color(egui::Color32::WHITE));
                                ui.separator();
                                ui.label(egui::RichText::new("Name").strong().color(egui::Color32::WHITE));
                                ui.separator();
                                ui.label(egui::RichText::new("Registered").strong().color(egui::Color32::WHITE));
                            });
                        });
                        ui.separator();
                        
                        // Display users in scrollable area
                        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                            for user in &self.registered_users {
                                let name = if let (Some(first), Some(last)) = (&user.first_name, &user.last_name) {
                                    format!("{} {}", first, last)
                                } else {
                                    "Unknown".to_string()
                                };
                                ui.horizontal(|ui| {
                                    ui.label(&user.username);
                                    ui.separator();
                                    ui.label(name);
                                    ui.separator();
                                    ui.label(&user.created_at);
                                });
                            }
                        });
                    } else {
                        let heading = match &self.selected_target {
                            ChatTarget::Lobby => "Lobby".to_string(),
                            ChatTarget::Direct(name) => name.clone(),
                        };
                        ui.heading(heading);
                        ui.separator();
                        
                        // Compact toolbar
                        ui.horizontal(|ui| {
                            ui.label("Search:");
                            let search_response = ui.add(
                                egui::TextEdit::singleline(&mut self.search_query)
                                    .hint_text("Find messages")
                                    .desired_width(200.0),
                            );
                            if ui.small_button("🔍").clicked() 
                                || (search_response.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                let query = self.search_query.trim();
                                if !query.is_empty() {
                                    self.search_in_progress = true;
                                    let _ = self.network.tx.send(UiToNet::Search {
                                        target: self.selected_target.clone(),
                                        query: query.to_string(),
                                    });
                                }
                            }
                            if ui.small_button("Clear").clicked() {
                                self.search_query.clear();
                                self.search_in_progress = false;
                                self.search_results.remove(&self.selected_target);
                            }
                            
                            let direct_name = match &self.selected_target {
                                ChatTarget::Direct(name) => Some(name.clone()),
                                _ => None,
                            };
                            if let Some(name) = direct_name {
                                ui.separator();
                                if ui.small_button("🚫 Block").clicked() {
                                    self.send_moderation(UiToNet::Block { username: name.clone() });
                                }
                                if ui.small_button("Unblock").clicked() {
                                    self.send_moderation(UiToNet::Unblock { username: name.clone() });
                                }
                                if ui.small_button("🔇 Mute").clicked() {
                                    self.send_moderation(UiToNet::Mute { username: name.clone() });
                                }
                                if ui.small_button("🔊 Unmute").clicked() {
                                    self.send_moderation(UiToNet::Unmute { username: name.clone() });
                                }
                                ui.separator();
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.report_reason)
                                        .hint_text("Report reason")
                                        .desired_width(120.0),
                                );
                                if ui.small_button("⚠️ Report").clicked() {
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
                            let relative = format_relative_time(&message.at);
                            if relative.is_empty() {
                                ui.label(format!("{}: {}", message.from, message.body));
                            } else {
                                ui.label(format!("[{relative}] {}: {}", message.from, message.body));
                            }
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
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.chat_input)
                                .hint_text("Type your message...")
                                .desired_width(f32::INFINITY),
                        );
                        if ui.button("Send").clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            self.send_chat();
                        }
                    });
                    }
                });
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
    visuals.window_rounding = egui::Rounding::same(6.0);
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

fn spawn_network() -> NetworkHandle {
    let (ui_tx, ui_rx) = mpsc::unbounded_channel::<UiToNet>();
    let (net_tx, net_rx) = std_mpsc::channel::<NetToUi>();
    let (update_tx, update_rx) = std_mpsc::channel::<String>();

    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("failed to create runtime");
        runtime.block_on(async move {
            network_task(ui_rx, net_tx).await;
        });
    });

    // Spawn update checker thread
    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("failed to create runtime");
        let _ = runtime.block_on(async {
            if let Ok(Some(version)) = update::check_for_updates().await {
                let _ = update_tx.send(version);
            }
        });
    });

    NetworkHandle { tx: ui_tx, rx: net_rx, update_rx }
}

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
                first_name,
                last_name,
                mode,
            } => {
                let result = run_connection(url, username, password, first_name, last_name, mode, &mut ui_rx, &net_tx).await;
                if let Err(message) = result {
                    let _ = net_tx.send(NetToUi::Error(message));
                }
            }
            UiToNet::Disconnect => {
                let _ = net_tx.send(NetToUi::Disconnected);
            }
            _ => {}
        }
    }
}

async fn run_connection(
    url: String,
    username: String,
    password: String,
    first_name: String,
    last_name: String,
    mode: AuthMode,
    ui_rx: &mut mpsc::UnboundedReceiver<UiToNet>,
    net_tx: &std_mpsc::Sender<NetToUi>,
) -> Result<(), String> {
    let (ws_stream, _) = connect_async(&url).await.map_err(|err| err.to_string())?;
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    match mode {
        AuthMode::Login => {
            send_json(&mut ws_tx, ClientToServer::Login { username, password }).await?;
        }
        AuthMode::Register => {
            send_json(&mut ws_tx, ClientToServer::Register { username, password, first_name, last_name }).await?;
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
                                    };
                                    let mapped_messages = messages
                                        .into_iter()
                                        .map(|record| ChatMessage {
                                            from: record.from,
                                            body: record.body,
                                            at: record.at,
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
                                    };
                                    let mapped_messages = messages
                                        .into_iter()
                                        .map(|record| ChatMessage {
                                            from: record.from,
                                            body: record.body,
                                            at: record.at,
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
                                ServerToClient::FriendAdded { username } => {
                                    let _ = net_tx.send(NetToUi::FriendAdded { username });
                                }
                                ServerToClient::FriendRequest { from } => {
                                    let _ = net_tx.send(NetToUi::FriendRequest { from });
                                }
                                ServerToClient::FriendRequestResponse { from, accepted } => {
                                    let _ = net_tx.send(NetToUi::FriendRequestResponse { from, accepted });
                                }
                                ServerToClient::Users { users } => {
                                    let _ = net_tx.send(NetToUi::Users { users });
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
                    UiToNet::FetchHistory { target } => {
                        let target = match target {
                            ChatTarget::Lobby => HistoryTarget::Lobby,
                            ChatTarget::Direct(username) => HistoryTarget::Direct { username },
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
                    UiToNet::AddFriend { username, nickname } => {
                        send_json(&mut ws_tx, ClientToServer::AddFriend { username, nickname }).await?;
                    }
                    UiToNet::FriendRequest { to } => {
                        send_json(&mut ws_tx, ClientToServer::FriendRequest { to }).await?;
                    }
                    UiToNet::RespondToFriendRequest { from, accepted } => {
                        send_json(&mut ws_tx, ClientToServer::RespondToFriendRequest { from, accepted }).await?;
                    }
                    UiToNet::GetUsers => {
                        send_json(&mut ws_tx, ClientToServer::GetUsers).await?;
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

fn download_and_install_update(version: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        download_and_install_windows(version)
    }
    
    #[cfg(target_os = "macos")]
    {
        download_and_install_macos(version)
    }
    
    #[cfg(target_os = "linux")]
    {
        Err("Auto-update not yet supported on Linux. Please download from GitHub releases.".to_string())
    }
}

#[cfg(target_os = "windows")]
fn download_and_install_windows(version: &str) -> Result<(), String> {
    // Download the installer to a temporary location
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join(format!("blast-from-the-past-messenger-update-{}.exe", version));
    
    let download_url = format!(
        "https://github.com/ravinathannur/chatmessagediscordclone/releases/download/{}/blast-from-the-past-messenger-setup.exe",
        version
    );
    
    // Download the installer
    eprintln!("Downloading update from: {}", download_url);
    let response = reqwest::blocking::Client::new()
        .get(&download_url)
        .send()
        .map_err(|e| format!("Failed to download: {}", e))?;
    
    let mut file = std::fs::File::create(&installer_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    std::io::copy(&mut response.bytes().map_err(|e| format!("Failed to read response: {}", e))?.as_ref(), &mut file)
        .map_err(|e| format!("Failed to write installer: {}", e))?;
    
    // Launch the installer
    eprintln!("Launching installer from: {:?}", installer_path);
    std::process::Command::new("cmd")
        .args(&["/C", installer_path.to_str().unwrap_or("")])
        .spawn()
        .map_err(|e| format!("Failed to launch installer: {}", e))?;
    
    Ok(())
}

#[cfg(target_os = "macos")]
fn download_and_install_macos(version: &str) -> Result<(), String> {
    
    // Download the DMG to a temporary location
    let temp_dir = std::env::temp_dir();
    let dmg_path = temp_dir.join(format!("BlastFromThePast-{}.dmg", version));
    
    let download_url = format!(
        "https://github.com/ravinathannur/chatmessagediscordclone/releases/download/{}/BlastFromThePast-{}.dmg",
        version, version
    );
    
    // Download the DMG
    eprintln!("Downloading update from: {}", download_url);
    let response = reqwest::blocking::Client::new()
        .get(&download_url)
        .send()
        .map_err(|e| format!("Failed to download: {}", e))?;
    
    let mut file = std::fs::File::create(&dmg_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    std::io::copy(&mut response.bytes().map_err(|e| format!("Failed to read response: {}", e))?.as_ref(), &mut file)
        .map_err(|e| format!("Failed to write DMG: {}", e))?;
    
    // Mount the DMG and open it
    eprintln!("Opening DMG from: {:?}", dmg_path);
    std::process::Command::new("open")
        .arg(&dmg_path)
        .spawn()
        .map_err(|e| format!("Failed to open DMG: {}", e))?;
    
    eprintln!("DMG opened. User will be guided to drag the app to Applications folder.");
    
    Ok(())
}

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

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(920.0, 640.0)),
        ..Default::default()
    };
    eframe::run_native(
        "AOL-Style Messenger",
        options,
        Box::new(|cc| Box::new(AolApp::new(cc))),
    )
}

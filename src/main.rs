use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;

use chrono::Utc;
use eframe::egui;

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

#[cfg(not(target_arch = "wasm32"))]
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
    AddFriendResult { _username: String, success: bool, message: String },
    FriendRequest { from: String },
    FriendRequestResult { username: String, accepted: bool },
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
    away_text: String,
    chat_input: String,
    dm_target: String,
    selected_target: ChatTarget,
    report_reason: String,
    logging_in: bool,
    login_started_at: Option<Instant>,
    login_frame_count: u32,  // Web: tracks elapsed frames for 3-second timeout
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
}

struct SearchResult {
    query: String,
    messages: Vec<ChatMessage>,
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
            away_text: String::new(),
            chat_input: String::new(),
            dm_target: String::new(),
            selected_target: ChatTarget::Lobby,
            report_reason: String::new(),
            logging_in: false,
            login_started_at: None,
            login_frame_count: 0,
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
                    });
                    self.audio_manager.play(SoundEffect::MessageReceived);
                }
                NetToUi::DirectMessage { from, body } => {
                    let target = ChatTarget::Direct(from.clone());
                    let entry = self.messages.entry(target).or_default();
                    entry.push(ChatMessage {
                        from,
                        body,
                        at: Utc::now().to_rfc3339(),
                    });
                    self.audio_manager.play(SoundEffect::MessageReceived);
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
            url = format!("wss://{url}");
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
        match &self.selected_target {
            ChatTarget::Lobby => {
                let _ = self
                    .network
                    .tx
                    .send(UiToNet::SendChat { body: body.clone() });
            }
            ChatTarget::Direct(target) => {
                let _ = self.network.tx.send(UiToNet::SendDirect {
                    to: target.clone(),
                    body: body.clone(),
                });
                let entry = self
                    .messages
                    .entry(ChatTarget::Direct(target.clone()))
                    .or_default();
                entry.push(ChatMessage {
                    from: self
                        .logged_in_user
                        .clone()
                        .unwrap_or_else(|| "Me".to_string()),
                    body,
                    at: Utc::now().to_rfc3339(),
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
        }
        self.process_net_events();
        
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
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let frame = if let Some(start) = self.login_started_at {
                                        let idx = ((start.elapsed().as_millis() / 200) % frames.len() as u128) as usize;
                                        frames[idx]
                                    } else {
                                        frames[0]
                                    };
                                    #[cfg(target_arch = "wasm32")]
                                    let frame = {
                                        let idx = ((self.login_frame_count / 12) as usize) % frames.len();  // ~12 frames per second animation
                                        frames[idx]
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
                        if let Some(name) = &self.logged_in_user {
                            ui.label(format!("User: {name}"));
                        }

                        ui.separator();
                        let online_count = self.buddies.iter().filter(|b| b.away.is_none()).count();
                        let away_count = self.buddies.len() - online_count;
                        ui.label(format!("👥 {} online, {} away", online_count, away_count));
                        if ui.button("Add Friend").clicked() {
                            self.show_add_friend_modal = true;
                        }
                        // Show friend requests button with pending count
                        let pending_count = self.pending_friend_requests.len();
                        let fr_label = if pending_count > 0 {
                            format!("Friend Requests ({})", pending_count)
                        } else {
                            "Friend Requests".to_string()
                        };
                        if ui.button(fr_label).clicked() {
                            self.show_friend_requests_modal = true;
                        }
                        if ui.button("Disconnect").clicked() {
                            let _ = self.network.tx.send(UiToNet::Disconnect);
                            self.screen = Screen::SignIn;
                            self.logged_in_user = None;
                            self.selected_target = ChatTarget::Lobby;
                        }

                        ui.separator();

                        // Audio settings menu
                        ui.menu_button("🔊 Audio", |ui| {
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
                    });
                    // Modal for Add Friend
                    if self.show_add_friend_modal {
                        egui::Window::new("Add Friend")
                            .collapsible(false)
                            .resizable(false)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                ui.label("Enter friend's screen name:");
                                ui.add(egui::TextEdit::singleline(&mut self.add_friend_name).hint_text("Screen name"));
                                ui.horizontal(|ui| {
                                    if ui.button("Add").clicked() {
                                        self.send_add_friend();
                                        self.show_add_friend_modal = false;
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.show_add_friend_modal = false;
                                    }
                                });
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
                    ui.horizontal(|ui| {
                        ui.label("Away message:");
                        ui.add(egui::TextEdit::singleline(&mut self.away_text).hint_text("Be right back..."));
                        if ui.button("Update").clicked() {
                            self.send_away();
                        }
                    });
                });

                egui::SidePanel::left("buddy_list")
                    .resizable(false)
                    .min_width(200.0)
                    .show(ctx, |ui| {
                        ui.heading("Buddy List");
                        ui.separator();
                        if ui
                            .selectable_label(self.selected_target == ChatTarget::Lobby, "Lobby")
                            .clicked()
                        {
                            self.select_target(ChatTarget::Lobby);
                        }
                        ui.add_space(8.0);
                        ui.label("Recent DMs");
                        let recent_threads = self.recent_threads.clone();
                        for name in recent_threads {
                            let target = ChatTarget::Direct(name.clone());
                            if ui.selectable_label(self.selected_target == target, &name).clicked() {
                                self.select_target(ChatTarget::Direct(name));
                            }
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
                            if ui.button("Open").clicked() {
                                let target = self.dm_target.trim();
                                if !target.is_empty() {
                                    self.select_target(ChatTarget::Direct(target.to_string()));
                                    self.dm_target.clear();
                                }
                            }
                        });
                        ui.add_space(8.0);

                        // Separate buddies by status
                        let buddies = self.buddies.clone();
                        let (online, away): (Vec<_>, Vec<_>) = buddies
                            .iter()
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
                                    // Small profile circle
                                    let color = username_to_color(&buddy.username);
                                    let initials = get_initials(&buddy.username);

                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(24.0, 24.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().circle_filled(rect.center(), 12.0, color);
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &initials,
                                        egui::FontId::proportional(10.0),
                                        egui::Color32::WHITE,
                                    );

                                    let target = ChatTarget::Direct(buddy.username.clone());
                                    if ui.selectable_label(
                                        self.selected_target == target,
                                        &buddy.username
                                    ).clicked() {
                                        self.select_target(ChatTarget::Direct(buddy.username.clone()));
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
                                    // Small profile circle
                                    let color = username_to_color(&buddy.username);
                                    let initials = get_initials(&buddy.username);

                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(24.0, 24.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().circle_filled(rect.center(), 12.0, color);
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &initials,
                                        egui::FontId::proportional(10.0),
                                        egui::Color32::WHITE,
                                    );

                                    let target = ChatTarget::Direct(buddy.username.clone());
                                    if ui.selectable_label(
                                        self.selected_target == target,
                                        &buddy.username
                                    ).clicked() {
                                        self.select_target(ChatTarget::Direct(buddy.username.clone()));
                                    }

                                    if let Some(ref away_msg) = buddy.away {
                                        ui.label(
                                            egui::RichText::new(format!("({})", away_msg))
                                                .italics()
                                                .small()
                                                .color(egui::Color32::GRAY)
                                        );
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
                                            ui.label(
                                                egui::RichText::new(format!("• {}", relative))
                                                    .small()
                                                    .color(egui::Color32::GRAY)
                                            );
                                        }
                                    });
                                    // Convert emoticons and display message
                                    let body_with_emoji = convert_emoticons(&message.body);
                                    ui.label(&body_with_emoji);
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
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.chat_input)
                                .hint_text("Type your message... (Ctrl+Enter to send)")
                                .desired_width(f32::INFINITY),
                        );

                        // Multiple ways to send:
                        // 1. Click Send button
                        // 2. Press Enter when not focused (lost focus)
                        // 3. Press Ctrl/Cmd+Enter while typing
                        let should_send = ui.button("Send").clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            || (response.has_focus()
                                && ui.input(|i| {
                                    i.key_pressed(egui::Key::Enter)
                                        && (i.modifiers.command || i.modifiers.ctrl)
                                }));

                        if should_send {
                            self.send_chat();
                            response.request_focus();
                        }

                        // Escape to clear input
                        if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.chat_input.clear();
                        }
                    });
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

// Convert classic AIM emoticons to emoji
fn convert_emoticons(text: &str) -> String {
    text.replace(":)", "😊")
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
    let (ws_stream, _) = connect_async(&url).await.map_err(|err| err.to_string())?;
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

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
                                ServerToClient::AddFriendResult { _username, success, message } => {
                                    let _ = net_tx.send(NetToUi::AddFriendResult { _username, success, message });
                                }
                                ServerToClient::FriendRequest { from } => {
                                    let _ = net_tx.send(NetToUi::FriendRequest { from });
                                }
                                ServerToClient::FriendRequestResult { username, accepted } => {
                                    let _ = net_tx.send(NetToUi::FriendRequestResult { username, accepted });
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
                    UiToNet::AddFriend { username } => {
                        send_json(&mut ws_tx, ClientToServer::AddFriend { username }).await?;
                    }
                    UiToNet::AcceptFriendRequest { username } => {
                        send_json(&mut ws_tx, ClientToServer::AcceptFriendRequest { username }).await?;
                    }
                    UiToNet::DeclineFriendRequest { username } => {
                        send_json(&mut ws_tx, ClientToServer::DeclineFriendRequest { username }).await?;
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
        Box::new(|cc| Box::new(AolApp::new(cc))),
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
                Box::new(|cc| Box::new(AolApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}

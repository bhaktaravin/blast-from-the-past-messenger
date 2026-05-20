use rust_embed::RustEmbed;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

#[derive(RustEmbed)]
#[folder = "assets/sounds"]
struct SoundAssets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEffect {
    BuddySignOn,
    BuddySignOff,
    MessageReceived,
    MessageSent,
    DoorSlam,
    Typing,
}

impl SoundEffect {
    pub fn default_filename(self) -> &'static str {
        match self {
            SoundEffect::BuddySignOn => "buddy-in.wav",
            SoundEffect::BuddySignOff => "buddy-out.wav",
            SoundEffect::MessageReceived => "message.wav",
            SoundEffect::MessageSent => "send.wav",
            SoundEffect::DoorSlam => "buddy-out.wav",
            SoundEffect::Typing => "send.wav",
        }
    }

    pub fn settings_key(self) -> &'static str {
        match self {
            SoundEffect::BuddySignOn => "buddy_sign_on",
            SoundEffect::BuddySignOff => "buddy_sign_off",
            SoundEffect::MessageReceived => "message_received",
            SoundEffect::MessageSent => "message_sent",
            SoundEffect::DoorSlam => "door_slam",
            SoundEffect::Typing => "typing",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SoundEffect::BuddySignOn => "Buddy sign on",
            SoundEffect::BuddySignOff => "Buddy sign off",
            SoundEffect::MessageReceived => "Message received",
            SoundEffect::MessageSent => "Message sent",
            SoundEffect::DoorSlam => "Door slam",
            SoundEffect::Typing => "Typing",
        }
    }

    pub const ALL: [SoundEffect; 6] = [
        SoundEffect::BuddySignOn,
        SoundEffect::BuddySignOff,
        SoundEffect::MessageReceived,
        SoundEffect::MessageSent,
        SoundEffect::DoorSlam,
        SoundEffect::Typing,
    ];
}

pub struct AudioManager {
    enabled: bool,
    volume: f32,
    /// Override built-in filename per effect (must exist in embedded assets).
    overrides: std::collections::HashMap<String, String>,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            enabled: true,
            volume: 0.8,
            overrides: std::collections::HashMap::new(),
        }
    }

    pub fn set_override(&mut self, effect: SoundEffect, filename: Option<String>) {
        let key = effect.settings_key().to_string();
        match filename {
            Some(f) if !f.is_empty() => {
                self.overrides.insert(key, f);
            }
            _ => {
                self.overrides.remove(&key);
            }
        }
    }

    pub fn get_override(&self, effect: SoundEffect) -> Option<&str> {
        self.overrides
            .get(effect.settings_key())
            .map(|s| s.as_str())
    }

    pub fn play(&self, effect: SoundEffect) {
        if !self.enabled {
            return;
        }
        let filename = self
            .overrides
            .get(effect.settings_key())
            .map(|s| s.as_str())
            .unwrap_or(effect.default_filename());

        let volume = if matches!(effect, SoundEffect::Typing) {
            self.volume * 0.3
        } else {
            self.volume
        };

        if let Some(data) = SoundAssets::get(filename) {
            play_wav_bytes(data.data.as_ref(), volume);
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn overrides(&self) -> &std::collections::HashMap<String, String> {
        &self.overrides
    }

    pub fn set_overrides(&mut self, overrides: std::collections::HashMap<String, String>) {
        self.overrides = overrides;
    }
}

fn play_wav_bytes(bytes: &[u8], volume: f32) {
    use base64::Engine as _;
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    let uri = format!("data:audio/wav;base64,{b64}");
    let audio = match web_sys::HtmlAudioElement::new() {
        Ok(a) => a,
        Err(_) => return,
    };
    let _ = audio.set_src(&uri);
    let _ = audio.set_volume(volume.clamp(0.0, 1.0) as f64);
    if let Some(body) = document.body() {
        let _ = body.append_child(&audio);
        let _ = audio.play();
        let audio_clone = audio.clone();
        let closure = Closure::wrap(Box::new(move || {
            let _ = audio_clone.remove();
        }) as Box<dyn FnMut()>);
        audio.set_onended(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }
}

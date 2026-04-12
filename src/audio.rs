use rodio::{Decoder, OutputStream, Sink};
use rust_embed::RustEmbed;
use std::io::Cursor;

#[derive(RustEmbed)]
#[folder = "assets/sounds"]
pub struct SoundAssets;

#[derive(Debug, Clone, Copy)]
pub enum SoundEffect {
    BuddySignOn,
    BuddySignOff,
    MessageReceived,
    MessageSent,
    DoorSlam,      // When someone blocks you
    Typing,        // Typing sound
}

pub struct AudioManager {
    enabled: bool,
    volume: f32,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            enabled: true,
            volume: 0.8,
        }
    }

    pub fn play(&self, effect: SoundEffect) {
        if !self.enabled {
            return;
        }

        let volume = self.volume;
        std::thread::spawn(move || {
            let filename = match effect {
                SoundEffect::BuddySignOn => "buddy-in.wav",
                SoundEffect::BuddySignOff => "buddy-out.wav",
                SoundEffect::MessageReceived => "message.wav",
                SoundEffect::MessageSent => "send.wav",
                SoundEffect::DoorSlam => "buddy-out.wav", // Reuse buddy-out for now
                SoundEffect::Typing => "send.wav", // Reuse send for now (quieter)
            };

            if let Some(sound_data) = SoundAssets::get(filename) {
                let data = sound_data.data.to_vec();
                if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
                    let cursor = Cursor::new(data);
                    if let Ok(source) = Decoder::new(cursor) {
                        if let Ok(sink) = Sink::try_new(&stream_handle) {
                            // Lower volume for typing sound
                            let adjusted_volume = if matches!(effect, SoundEffect::Typing) {
                                volume * 0.3
                            } else {
                                volume
                            };
                            sink.set_volume(adjusted_volume);
                            sink.append(source);
                            sink.sleep_until_end();
                        }
                    }
                }
            }
        });
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
}

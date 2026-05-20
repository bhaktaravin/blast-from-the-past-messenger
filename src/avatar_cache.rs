//! Async avatar image loading and texture cache for buddy list / profiles.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use base64::Engine as _;

const MAX_AVATAR_PX: u32 = 128;

pub struct AvatarCache {
    textures: HashMap<String, TextureHandle>,
    pending: HashSet<String>,
    failed: HashSet<String>,
    fetch_rx: Receiver<(String, Vec<u8>)>,
    fetch_tx: Sender<(String, Vec<u8>)>,
}

impl AvatarCache {
    pub fn new() -> Self {
        let (fetch_tx, fetch_rx) = mpsc::channel();
        Self {
            textures: HashMap::new(),
            pending: HashSet::new(),
            failed: HashSet::new(),
            fetch_rx,
            fetch_tx,
        }
    }

    /// Apply completed downloads and create GPU textures.
    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok((key, bytes)) = self.fetch_rx.try_recv() {
            self.pending.remove(&key);
            match bytes_to_color_image(&bytes) {
                Some(image) => {
                    let tex = ctx.load_texture(
                        format!("avatar_{key}"),
                        image,
                        TextureOptions::LINEAR,
                    );
                    self.failed.remove(&key);
                    self.textures.insert(key, tex);
                    ctx.request_repaint();
                }
                None => {
                    self.failed.insert(key);
                }
            }
        }
    }

    pub fn request_load(&mut self, source: &str) {
        let key = source.to_string();
        if self.textures.contains_key(&key)
            || self.pending.contains(&key)
            || self.failed.contains(&key)
        {
            return;
        }

        if let Some(bytes) = decode_inline_image(source) {
            self.pending.insert(key.clone());
            let _ = self.fetch_tx.send((key, bytes));
            return;
        }

        if !(source.starts_with("http://") || source.starts_with("https://")) {
            self.failed.insert(key);
            return;
        }

        self.pending.insert(key.clone());
        let url = source.to_string();
        let tx = self.fetch_tx.clone();

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                if let Some(bytes) = fetch_url_blocking(&url) {
                    let _ = tx.send((url, bytes));
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(bytes) = fetch_url_async(&url).await {
                    let _ = tx.send((url, bytes));
                }
            });
        }
    }

    /// Draw avatar image in `rect` if loaded; returns true if an image was drawn.
    pub fn paint_image(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        source: &str,
    ) -> bool {
        self.poll(ctx);
        if !self.textures.contains_key(source) && !self.pending.contains(source) {
            self.request_load(source);
        }

        if let Some(tex) = self.textures.get(source) {
            ui.painter().image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            true
        } else {
            false
        }
    }
}

fn bytes_to_color_image(bytes: &[u8]) -> Option<ColorImage> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = img.thumbnail(MAX_AVATAR_PX, MAX_AVATAR_PX);
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// Decode `data:image/...;base64,...` or raw base64 stored in the DB.
fn decode_inline_image(source: &str) -> Option<Vec<u8>> {
    if let Some(rest) = source.strip_prefix("data:image") {
        let b64 = rest.split(',').nth(1)?;
        return base64::engine::general_purpose::STANDARD.decode(b64).ok();
    }
    if source.len() > 80 && source.chars().all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c)) {
        return base64::engine::general_purpose::STANDARD.decode(source).ok();
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_url_blocking(url: &str) -> Option<Vec<u8>> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("blast-from-the-past-messenger")
        .build()
        .ok()?
        .get(url)
        .send()
        .ok()?
        .bytes()
        .ok()
        .map(|b| b.to_vec())
}

#[cfg(target_arch = "wasm32")]
async fn fetch_url_async(url: &str) -> Option<Vec<u8>> {
    gloo_net::http::Request::get(url)
        .send()
        .await
        .ok()?
        .binary()
        .await
        .ok()
}

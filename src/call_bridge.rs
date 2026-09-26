//! Bridge to the browser-side WebRTC call manager (`window.bfCall` in index.html).
//!
//! Media never touches Rust: the page owns the camera, the RTCPeerConnection and the
//! call overlay, and we relay its signaling to and from the server. Native builds
//! have no WebRTC stack, so every call here is a no-op there and `SUPPORTED` is false.

use serde::Deserialize;

pub const SUPPORTED: bool = cfg!(target_arch = "wasm32");

/// Something the page wants the app to act on.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CallEvent {
    /// SDP or ICE candidate to relay to the other party
    Signal { call_id: String, data: String },
    /// User pressed hang up in the call overlay
    Hangup { call_id: String },
    /// Call couldn't continue (camera denied, network blocked, ...)
    Error { call_id: String, message: String },
}

#[derive(Clone, Copy)]
pub enum Ring {
    /// Phone ringing — someone is calling us
    Incoming,
    /// Ringback tone while we wait for them to pick up
    Outgoing,
}

#[cfg(target_arch = "wasm32")]
mod js {
    use wasm_bindgen::prelude::*;

    // `catch` so a missing/broken page script surfaces as an Err instead of a wasm trap
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(catch, js_namespace = bfCall, js_name = start)]
        pub fn start(call_id: &str, role: &str, ice_servers: &str, peer: &str) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = bfCall, js_name = signal)]
        pub fn signal(call_id: &str, data: &str) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = bfCall, js_name = end)]
        pub fn end() -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = bfCall, js_name = ring)]
        pub fn ring(kind: &str) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = bfCall, js_name = takeEvents)]
        pub fn take_events() -> Result<String, JsValue>;
    }
}

/// Open the call overlay and start media. The caller sends the offer; the callee waits for it.
pub fn start(call_id: &str, is_caller: bool, ice_servers: &str, peer: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let role = if is_caller { "caller" } else { "callee" };
        let _ = js::start(call_id, role, ice_servers, peer);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (call_id, is_caller, ice_servers, peer);
}

/// Hand signaling from the other party to the page.
pub fn signal(call_id: &str, data: &str) {
    #[cfg(target_arch = "wasm32")]
    let _ = js::signal(call_id, data);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (call_id, data);
}

/// Tear down media and hide the overlay. Safe to call when no call is running.
pub fn end() {
    #[cfg(target_arch = "wasm32")]
    let _ = js::end();
}

/// Start a ring tone, or stop it with `None`.
pub fn ring(kind: Option<Ring>) {
    #[cfg(target_arch = "wasm32")]
    {
        let kind = match kind {
            Some(Ring::Incoming) => "incoming",
            Some(Ring::Outgoing) => "outgoing",
            None => "",
        };
        let _ = js::ring(kind);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = kind;
}

/// Drain events the page queued since the last frame.
pub fn take_events() -> Vec<CallEvent> {
    #[cfg(target_arch = "wasm32")]
    {
        js::take_events()
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    Vec::new()
}

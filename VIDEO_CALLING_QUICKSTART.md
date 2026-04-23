# 🎥 Video Calling - Quick Start

## Let's Start Simple!

Instead of building everything at once, let's start with a **minimal working prototype** that you can test quickly.

## Phase 1: Web-Only Prototype (Easiest!)

Start with the web version since browser WebRTC APIs are simpler and well-documented.

### Step 1: Add Protocol Messages (5 minutes)

Add to `src/protocol.rs`:

```rust
// Add these to ClientToServer enum
CallUser { to: String },
AcceptCall { from: String },
DeclineCall { from: String },
WebRtcSignal { to: String, signal: String },

// Add these to ServerToClient enum
IncomingCall { from: String },
CallAccepted { peer: String },
CallDeclined { peer: String },
WebRtcSignal { from: String, signal: String },
```

### Step 2: Update Server (10 minutes)

Add to `src/bin/server.rs`:

```rust
// Just relay the messages
ClientToServer::CallUser { to } => {
    if let Some(tx) = connections.get(&to) {
        let _ = tx.send(ServerToClient::IncomingCall {
            from: username.clone(),
        });
    }
}

ClientToServer::AcceptCall { from } => {
    if let Some(tx) = connections.get(&from) {
        let _ = tx.send(ServerToClient::CallAccepted {
            peer: username.clone(),
        });
    }
}

ClientToServer::WebRtcSignal { to, signal } => {
    if let Some(tx) = connections.get(&to) {
        let _ = tx.send(ServerToClient::WebRtcSignal {
            from: username.clone(),
            signal,
        });
    }
}
```

### Step 3: Add Web Dependencies (2 minutes)

Update `Cargo.toml`:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "0.3", features = [
    # ... existing features ...
    "MediaStream",
    "MediaStreamConstraints",
    "MediaDevices",
    "Navigator",
    "RtcPeerConnection",
    "RtcConfiguration",
    "RtcSessionDescription",
    "RtcSessionDescriptionInit",
    "RtcSdpType",
    "RtcIceCandidate",
    "RtcIceCandidateInit",
    "VideoElement",
    "HtmlVideoElement",
    "HtmlCanvasElement",
    "CanvasRenderingContext2d",
] }
js-sys = "0.3"
```

### Step 4: Create Simple Video Module (30 minutes)

Create `src/video_simple.rs`:

```rust
#[cfg(target_arch = "wasm32")]
pub mod video {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::*;
    
    pub struct SimpleVideoCall {
        peer_connection: Option<RtcPeerConnection>,
        local_stream: Option<MediaStream>,
    }
    
    impl SimpleVideoCall {
        pub fn new() -> Self {
            Self {
                peer_connection: None,
                local_stream: None,
            }
        }
        
        pub async fn start_camera(&mut self) -> Result<(), JsValue> {
            let window = web_sys::window().unwrap();
            let navigator = window.navigator();
            let media_devices = navigator.media_devices()?;
            
            let mut constraints = MediaStreamConstraints::new();
            constraints.video(&JsValue::TRUE);
            constraints.audio(&JsValue::TRUE);
            
            let promise = media_devices.get_user_media_with_constraints(&constraints)?;
            let stream_js = wasm_bindgen_futures::JsFuture::from(promise).await?;
            let stream: MediaStream = stream_js.dyn_into()?;
            
            self.local_stream = Some(stream);
            Ok(())
        }
        
        pub fn show_local_video(&self, video_id: &str) -> Result<(), JsValue> {
            if let Some(stream) = &self.local_stream {
                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                
                if let Some(video_element) = document.get_element_by_id(video_id) {
                    let video: HtmlVideoElement = video_element.dyn_into()?;
                    video.set_src_object(Some(stream));
                    let _ = video.play()?;
                }
            }
            Ok(())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod video {
    pub struct SimpleVideoCall;
    
    impl SimpleVideoCall {
        pub fn new() -> Self {
            Self
        }
    }
}
```

### Step 5: Add UI Button (10 minutes)

In `src/main.rs`, add to buddy list context menu:

```rust
if ui.button("📹 Start Video Call").clicked() {
    // For now, just show a message
    self.show_toast(
        format!("Video calling {} (coming soon!)", buddy.username),
        ToastKind::Info
    );
    ui.close_menu();
}
```

### Step 6: Test Camera Access (5 minutes)

Build and test:

```bash
trunk serve
```

Open browser, click the video call button, and check if camera permission is requested.

## Phase 2: Simple P2P Connection (Next)

Once Phase 1 works, we'll add:
1. WebRTC peer connection
2. Offer/Answer exchange
3. ICE candidates
4. Video streaming

## Why Start Simple?

1. **Test infrastructure first** - Make sure signaling works
2. **Iterate quickly** - See results fast
3. **Learn as you go** - WebRTC is complex, start small
4. **Debug easier** - Fewer moving parts

## Quick Wins

After Phase 1, you'll have:
- ✅ Call signaling working
- ✅ Camera access working
- ✅ UI for video calls
- ✅ Server relay working

Then we can add the actual video streaming!

## Alternative: Use a Library

If you want to move faster, consider using a WebRTC library:

### Option 1: Simple-Peer (JavaScript)
Use via wasm-bindgen:
```javascript
// In your HTML
<script src="https://cdn.jsdelivr.net/npm/simple-peer@9/simplepeer.min.js"></script>
```

### Option 2: PeerJS
Even simpler, handles signaling too:
```javascript
<script src="https://unpkg.com/peerjs@1.5.0/dist/peerjs.min.js"></script>
```

### Option 3: Daily.co / Agora / Twilio
Managed video calling services (easiest but costs money):
- **Daily.co**: Free for <10 participants
- **Agora**: Free tier available
- **Twilio**: Pay as you go

## My Recommendation

**Start with Phase 1** to get the infrastructure working, then decide:

1. **DIY WebRTC**: Full control, learning experience, free
2. **Use Library**: Faster, less code, still free
3. **Managed Service**: Fastest, most reliable, costs money

Want to start with Phase 1? I can help you implement it step by step!

# 🎥 Video Calling Implementation Guide

## Overview

We'll add peer-to-peer video calling using WebRTC. This will work on both native desktop and web versions!

## Architecture

```
User A                    Server (Railway)              User B
  |                            |                           |
  |-- Call Request ----------->|                           |
  |                            |-- Incoming Call --------->|
  |                            |                           |
  |<-- Accept/Decline ---------|<-- Accept ----------------|
  |                            |                           |
  |-- WebRTC Offer ----------->|-- Forward Offer --------->|
  |<-- WebRTC Answer -----------|<-- Answer ----------------|
  |                            |                           |
  |<========== Direct P2P Video Connection =============>|
  |                            |                           |
  |-- ICE Candidates --------->|-- Forward ICE ----------->|
  |<-- ICE Candidates ----------|<-- ICE ------------------|
```

**Key Points:**
- Server only handles signaling (call setup)
- Actual video/audio goes peer-to-peer (P2P)
- Falls back to TURN server if P2P fails

## Technology Stack

### For Rust Native App:
- **webrtc-rs** - WebRTC implementation in Rust
- **tokio** - Async runtime (already using)
- **egui** - UI integration (already using)

### For Web (WASM):
- **web-sys** - Browser WebRTC APIs
- **wasm-bindgen** - JS interop

### Server:
- WebSocket for signaling (already have!)
- Optional: TURN server for NAT traversal

## Step-by-Step Implementation

### Phase 1: Basic Infrastructure (Week 1)

#### 1.1 Add Dependencies

```toml
# Cargo.toml
[dependencies]
# Native only
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
webrtc = "0.9"
tokio-stream = "0.1"

# Web only  
[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "0.3", features = [
    "MediaStream",
    "MediaStreamConstraints",
    "MediaDevices",
    "Navigator",
    "RtcPeerConnection",
    "RtcConfiguration",
    "RtcSessionDescription",
    "RtcIceCandidate",
    "RtcDataChannel",
    "VideoElement",
    "HtmlVideoElement",
] }
```

#### 1.2 Update Protocol

Add to `src/protocol.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientToServer {
    // ... existing variants ...
    
    // Video calling
    CallUser { to: String },
    AcceptCall { from: String },
    DeclineCall { from: String },
    EndCall { peer: String },
    WebRtcOffer { to: String, sdp: String },
    WebRtcAnswer { to: String, sdp: String },
    IceCandidate { to: String, candidate: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToClient {
    // ... existing variants ...
    
    // Video calling
    IncomingCall { from: String },
    CallAccepted { peer: String },
    CallDeclined { peer: String },
    CallEnded { peer: String },
    WebRtcOffer { from: String, sdp: String },
    WebRtcAnswer { from: String, sdp: String },
    IceCandidate { from: String, candidate: String },
}
```

### Phase 2: Server-Side Signaling (Week 1)

Update `src/bin/server.rs` to relay WebRTC messages:

```rust
// In handle_client function
ClientToServer::CallUser { to } => {
    // Find target user's connection
    if let Some(target_tx) = connections.get(&to) {
        let _ = target_tx.send(ServerToClient::IncomingCall {
            from: username.clone(),
        });
    }
}

ClientToServer::AcceptCall { from } => {
    if let Some(caller_tx) = connections.get(&from) {
        let _ = caller_tx.send(ServerToClient::CallAccepted {
            peer: username.clone(),
        });
    }
}

ClientToServer::WebRtcOffer { to, sdp } => {
    if let Some(target_tx) = connections.get(&to) {
        let _ = target_tx.send(ServerToClient::WebRtcOffer {
            from: username.clone(),
            sdp,
        });
    }
}

ClientToServer::WebRtcAnswer { to, sdp } => {
    if let Some(target_tx) = connections.get(&to) {
        let _ = target_tx.send(ServerToClient::WebRtcAnswer {
            from: username.clone(),
            sdp,
        });
    }
}

ClientToServer::IceCandidate { to, candidate } => {
    if let Some(target_tx) = connections.get(&to) {
        let _ = target_tx.send(ServerToClient::IceCandidate {
            from: username.clone(),
            candidate,
        });
    }
}
```

### Phase 3: Native Client Implementation (Week 2)

Create `src/video_call.rs`:

```rust
#[cfg(not(target_arch = "wasm32"))]
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::ice_transport::ice_server::RTCIceServer;

pub struct VideoCall {
    peer_connection: Option<Arc<RTCPeerConnection>>,
    local_stream: Option<MediaStream>,
    remote_stream: Option<MediaStream>,
    peer_username: String,
}

impl VideoCall {
    pub async fn new(peer_username: String) -> Result<Self, Box<dyn std::error::Error>> {
        // Create MediaEngine
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs()?;
        
        // Create API
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .build();
        
        // ICE servers (STUN for NAT traversal)
        let config = RTCConfiguration {
            ice_servers: vec![
                RTCIceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_string()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        
        // Create peer connection
        let peer_connection = Arc::new(api.new_peer_connection(config).await?);
        
        Ok(Self {
            peer_connection: Some(peer_connection),
            local_stream: None,
            remote_stream: None,
            peer_username,
        })
    }
    
    pub async fn start_local_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Get user media (camera + microphone)
        // This is platform-specific and requires additional crates
        // For now, we'll use a placeholder
        
        // TODO: Implement camera/mic capture
        // On Linux: v4l2, PulseAudio
        // On macOS: AVFoundation
        // On Windows: DirectShow
        
        Ok(())
    }
    
    pub async fn create_offer(&self) -> Result<String, Box<dyn std::error::Error>> {
        let pc = self.peer_connection.as_ref().unwrap();
        let offer = pc.create_offer(None).await?;
        pc.set_local_description(offer.clone()).await?;
        
        Ok(offer.sdp)
    }
    
    pub async fn create_answer(&self, offer_sdp: String) -> Result<String, Box<dyn std::error::Error>> {
        let pc = self.peer_connection.as_ref().unwrap();
        
        // Set remote description (offer)
        let offer = RTCSessionDescription::offer(offer_sdp)?;
        pc.set_remote_description(offer).await?;
        
        // Create answer
        let answer = pc.create_answer(None).await?;
        pc.set_local_description(answer.clone()).await?;
        
        Ok(answer.sdp)
    }
    
    pub async fn set_remote_answer(&self, answer_sdp: String) -> Result<(), Box<dyn std::error::Error>> {
        let pc = self.peer_connection.as_ref().unwrap();
        let answer = RTCSessionDescription::answer(answer_sdp)?;
        pc.set_remote_description(answer).await?;
        Ok(())
    }
    
    pub async fn add_ice_candidate(&self, candidate: String) -> Result<(), Box<dyn std::error::Error>> {
        let pc = self.peer_connection.as_ref().unwrap();
        // Parse and add ICE candidate
        // TODO: Implement ICE candidate parsing
        Ok(())
    }
    
    pub async fn close(&mut self) {
        if let Some(pc) = &self.peer_connection {
            let _ = pc.close().await;
        }
        self.peer_connection = None;
        self.local_stream = None;
        self.remote_stream = None;
    }
}
```

### Phase 4: Web Client Implementation (Week 2)

Create `src/video_call_web.rs`:

```rust
#[cfg(target_arch = "wasm32")]
use web_sys::{
    MediaStream, MediaStreamConstraints, RtcPeerConnection,
    RtcConfiguration, RtcSessionDescription, RtcIceCandidate,
    HtmlVideoElement,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub struct VideoCallWeb {
    peer_connection: Option<RtcPeerConnection>,
    local_stream: Option<MediaStream>,
    peer_username: String,
}

impl VideoCallWeb {
    pub async fn new(peer_username: String) -> Result<Self, JsValue> {
        // Create RTCPeerConnection
        let mut config = RtcConfiguration::new();
        
        // Add STUN server
        let ice_servers = js_sys::Array::new();
        let stun_server = js_sys::Object::new();
        js_sys::Reflect::set(
            &stun_server,
            &"urls".into(),
            &"stun:stun.l.google.com:19302".into(),
        )?;
        ice_servers.push(&stun_server);
        config.ice_servers(&ice_servers);
        
        let peer_connection = RtcPeerConnection::new_with_configuration(&config)?;
        
        Ok(Self {
            peer_connection: Some(peer_connection),
            local_stream: None,
            peer_username,
        })
    }
    
    pub async fn start_local_stream(&mut self) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();
        let navigator = window.navigator();
        let media_devices = navigator.media_devices()?;
        
        // Request camera and microphone
        let mut constraints = MediaStreamConstraints::new();
        constraints.video(&JsValue::TRUE);
        constraints.audio(&JsValue::TRUE);
        
        let promise = media_devices.get_user_media_with_constraints(&constraints)?;
        let stream = wasm_bindgen_futures::JsFuture::from(promise).await?;
        let media_stream: MediaStream = stream.dyn_into()?;
        
        // Add tracks to peer connection
        if let Some(pc) = &self.peer_connection {
            let tracks = media_stream.get_tracks();
            for i in 0..tracks.length() {
                if let Some(track) = tracks.get(i) {
                    pc.add_track_0(&track.dyn_into()?, &media_stream);
                }
            }
        }
        
        self.local_stream = Some(media_stream);
        Ok(())
    }
    
    pub async fn create_offer(&self) -> Result<String, JsValue> {
        let pc = self.peer_connection.as_ref().unwrap();
        
        let offer = wasm_bindgen_futures::JsFuture::from(
            pc.create_offer()
        ).await?;
        
        let offer_desc: RtcSessionDescription = offer.dyn_into()?;
        
        wasm_bindgen_futures::JsFuture::from(
            pc.set_local_description(&offer_desc)
        ).await?;
        
        Ok(offer_desc.sdp())
    }
    
    pub async fn create_answer(&self, offer_sdp: String) -> Result<String, JsValue> {
        let pc = self.peer_connection.as_ref().unwrap();
        
        let mut offer_desc = RtcSessionDescription::new("offer")?;
        offer_desc.set_sdp(&offer_sdp);
        
        wasm_bindgen_futures::JsFuture::from(
            pc.set_remote_description(&offer_desc)
        ).await?;
        
        let answer = wasm_bindgen_futures::JsFuture::from(
            pc.create_answer()
        ).await?;
        
        let answer_desc: RtcSessionDescription = answer.dyn_into()?;
        
        wasm_bindgen_futures::JsFuture::from(
            pc.set_local_description(&answer_desc)
        ).await?;
        
        Ok(answer_desc.sdp())
    }
    
    pub fn attach_to_video_element(&self, video_id: &str) -> Result<(), JsValue> {
        if let Some(stream) = &self.local_stream {
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let video: HtmlVideoElement = document
                .get_element_by_id(video_id)
                .unwrap()
                .dyn_into()?;
            
            video.set_src_object(Some(stream));
            let _ = video.play()?;
        }
        Ok(())
    }
}
```

### Phase 5: UI Integration (Week 3)

Add to `src/main.rs`:

```rust
struct AolApp {
    // ... existing fields ...
    
    // Video call state
    active_call: Option<VideoCall>,
    incoming_call_from: Option<String>,
    show_video_window: bool,
}

impl AolApp {
    fn render_video_call_ui(&mut self, ctx: &egui::Context) {
        // Incoming call modal
        if let Some(ref caller) = self.incoming_call_from {
            egui::Window::new("📞 Incoming Call")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading(format!("{} is calling...", caller));
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("✅ Accept").clicked() {
                            self.accept_call(caller.clone());
                        }
                        if ui.button("❌ Decline").clicked() {
                            self.decline_call(caller.clone());
                        }
                    });
                });
        }
        
        // Active call window
        if self.show_video_window {
            egui::Window::new("📹 Video Call")
                .default_size([640.0, 480.0])
                .resizable(true)
                .show(ctx, |ui| {
                    // Video preview area
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 360.0),
                        egui::Sense::hover(),
                    );
                    
                    // Draw video placeholder
                    ui.painter().rect_filled(
                        rect,
                        0.0,
                        egui::Color32::from_rgb(20, 20, 20),
                    );
                    
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "📹 Video Stream",
                        egui::FontId::proportional(24.0),
                        egui::Color32::WHITE,
                    );
                    
                    ui.add_space(10.0);
                    
                    // Controls
                    ui.horizontal(|ui| {
                        if ui.button("🔇 Mute").clicked() {
                            // Toggle mute
                        }
                        if ui.button("📹 Camera Off").clicked() {
                            // Toggle camera
                        }
                        if ui.button("🔴 End Call").clicked() {
                            self.end_call();
                        }
                    });
                });
        }
    }
    
    fn start_call(&mut self, username: String) {
        let _ = self.network.tx.send(UiToNet::CallUser { to: username });
    }
    
    fn accept_call(&mut self, from: String) {
        let _ = self.network.tx.send(UiToNet::AcceptCall { from });
        self.incoming_call_from = None;
        self.show_video_window = true;
        
        // Initialize WebRTC
        // TODO: Create VideoCall instance and start stream
    }
    
    fn decline_call(&mut self, from: String) {
        let _ = self.network.tx.send(UiToNet::DeclineCall { from });
        self.incoming_call_from = None;
    }
    
    fn end_call(&mut self) {
        if let Some(call) = &self.active_call {
            let _ = self.network.tx.send(UiToNet::EndCall {
                peer: call.peer_username.clone(),
            });
        }
        self.active_call = None;
        self.show_video_window = false;
    }
}
```

Add call button to buddy list context menu:

```rust
// In buddy list rendering
username_response.context_menu(|ui| {
    // ... existing menu items ...
    
    if ui.button("📹 Video Call").clicked() {
        self.start_call(buddy.username.clone());
        ui.close_menu();
    }
});
```

### Phase 6: STUN/TURN Server Setup (Week 3)

For production, you'll need TURN servers for NAT traversal:

#### Option 1: Use Free STUN Servers
```rust
let ice_servers = vec![
    "stun:stun.l.google.com:19302",
    "stun:stun1.l.google.com:19302",
    "stun:stun2.l.google.com:19302",
];
```

#### Option 2: Self-Host TURN Server (coturn)

```bash
# Install coturn
sudo apt-get install coturn

# Configure /etc/turnserver.conf
listening-port=3478
fingerprint
lt-cred-mech
user=username:password
realm=yourdomain.com
```

Then use in your app:
```rust
let ice_servers = vec![
    RTCIceServer {
        urls: vec!["stun:yourdomain.com:3478".to_string()],
        ..Default::default()
    },
    RTCIceServer {
        urls: vec!["turn:yourdomain.com:3478".to_string()],
        username: Some("username".to_string()),
        credential: Some("password".to_string()),
        ..Default::default()
    },
];
```

#### Option 3: Use Managed Service
- **Twilio STUN/TURN**: https://www.twilio.com/stun-turn
- **Xirsys**: https://xirsys.com/
- **Metered**: https://www.metered.ca/stun-turn

## Testing Strategy

### Local Testing
1. Run two instances of the app
2. Test call initiation
3. Test accept/decline
4. Test video/audio streams
5. Test end call

### Network Testing
1. Test on same network
2. Test across different networks
3. Test with firewall/NAT
4. Test connection quality

### Browser Testing (Web version)
1. Chrome/Chromium
2. Firefox
3. Safari
4. Edge

## Challenges & Solutions

### Challenge 1: Camera/Mic Access (Native)
**Solution**: Use platform-specific crates:
- Linux: `v4l` + `cpal`
- macOS: `coreaudio-rs`
- Windows: `windows-rs`

### Challenge 2: Video Rendering (Native)
**Solution**: 
- Use `egui` texture for video frames
- Or use separate window with `winit` + `wgpu`

### Challenge 3: NAT Traversal
**Solution**:
- Use STUN for most cases
- Use TURN as fallback
- Implement ICE properly

### Challenge 4: Codec Support
**Solution**:
- VP8/VP9 for video (widely supported)
- Opus for audio (best quality)
- H.264 as fallback

## Performance Considerations

- **Video Resolution**: Start with 640x480, allow HD
- **Frame Rate**: 24-30 FPS
- **Bitrate**: Adaptive based on connection
- **CPU Usage**: Hardware encoding when available

## Security Considerations

- **DTLS**: Enabled by default in WebRTC
- **SRTP**: Encrypted media streams
- **Permissions**: Request camera/mic permission
- **Privacy**: Show indicator when camera is on

## Cost Estimate

### Self-Hosted TURN Server
- **VPS**: $5-10/month (DigitalOcean, Linode)
- **Bandwidth**: ~1GB per hour of video call
- **For 100 users**: ~$20-30/month

### Managed TURN Service
- **Twilio**: $0.0004/minute
- **Xirsys**: $10/month for 10GB
- **Metered**: Pay as you go

## Timeline

- **Week 1**: Protocol + Server signaling
- **Week 2**: Native WebRTC implementation
- **Week 3**: Web WebRTC + UI
- **Week 4**: Testing + Polish
- **Week 5**: TURN server + Production deploy

## Next Steps

1. Start with protocol changes
2. Implement server signaling
3. Build web version first (easier to test)
4. Add native support
5. Deploy TURN server
6. Test extensively

## Resources

- [WebRTC for the Curious](https://webrtcforthecurious.com/)
- [webrtc-rs Documentation](https://github.com/webrtc-rs/webrtc)
- [MDN WebRTC API](https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API)
- [coturn Server](https://github.com/coturn/coturn)

---

Ready to start? I can help you implement any phase! Let me know which part you want to tackle first.

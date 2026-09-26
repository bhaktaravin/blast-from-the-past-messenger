# Video Calling

1:1 video calls between buddies, built on WebRTC. Video and audio flow directly
between the two browsers; the chat server only relays the short setup messages
("signaling") and never sees the media.

## Using it

- Right-click a buddy → **📹 Video Call**, or press **📹 Video Call** at the top of a DM window.
- The other person gets a ringing **Incoming Video Call** window with **Answer** / **Decline**.
- Unanswered calls stop ringing after 45 seconds and show as a missed call.
- In a call: mute, camera on/off, switch camera (phones with more than one), hang up.

Calls are **web only** for now. The desktop app shows a toast when someone calls
you, so you can answer from the web version.

## How it works

```
Caller (browser)            Chat server                 Callee (browser)
  call_invite ───────────────▶ creates call ─────────────▶ incoming_call   (rings)
  call_ringing ◀──────────────┘
                                              ◀──────────── call_answer (accept)
  call_accepted ◀─────────────┘
  call_signal (SDP offer) ────▶ relays ─────────────────────▶
  ◀──────────────────────────── relays ◀──── call_signal (SDP answer, ICE)
  ════════════════ media flows peer-to-peer (STUN / TURN) ════════════════
  call_hangup ────────────────▶ call_ended ───────────────▶
```

| Piece | Where |
|---|---|
| Messages | `src/protocol.rs` (`Call*` variants) |
| Call state, busy/offline checks, ring timeout, disconnect cleanup | `src/bin/server.rs` (`CallHub`, `finish_call`) |
| Ringing/dialing windows, call state machine | `src/main.rs` (`CallPhase`, `draw_call_ui`) |
| Rust ↔ page bridge | `src/call_bridge.rs` |
| Camera, `RTCPeerConnection`, call overlay, ring tones | `index.html` (`window.bfCall`) |

`call_ended` reasons: `declined`, `busy`, `offline`, `unavailable` (unknown user, or
the callee blocked/muted you), `rate_limited`, `no_answer`, `hangup`,
`answered_elsewhere`, `disconnected`.

## Connectivity: STUN and TURN

By default the server hands out free public STUN servers (Google, Cloudflare).
That connects most calls. Some networks (strict corporate Wi-Fi, some mobile
carriers) block direct connections; those calls need a **TURN relay**. Without
one, those calls fail with "Couldn't connect the call — the network may be blocking it."

To add TURN, set `ICE_SERVERS` on the server to a JSON array of
[RTCIceServer](https://developer.mozilla.org/en-US/docs/Web/API/RTCIceServer) entries:

```bash
ICE_SERVERS='[{"urls":"stun:stun.cloudflare.com:3478"},{"urls":["turn:turn.example.com:3478?transport=udp","turns:turn.example.com:443?transport=tcp"],"username":"user","credential":"pass"}]'
```

## Testing locally

Browsers only allow camera access on **secure pages**: `https://`, or
`http://localhost`. So:

- **Same computer:** `./run-server.sh`, then `trunk serve`, and open two browser
  windows at `http://localhost:8080` (use a private window for the second user).
  Set the server URL on the login screen to `ws://localhost:9001`.
- **Phone:** a plain `http://<lan-ip>:8080` page can't use the camera. Test on the
  deployed (https) site, or serve over https.

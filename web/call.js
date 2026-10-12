// 1:1 video calls over WebRTC. Media flows directly between the browsers; the
// signaling goes through the chat server, relayed by whoever drives this module:
//  - the web app (src/call_bridge.rs), which calls start/signal/end/ring and
//    drains takeEvents() every frame
//  - call.html, the page the desktop apps open in a browser for their calls
//
// Load it at the end of <body>, after call.css; it adds the overlay markup itself.

const OVERLAY_HTML = `
<div id="call-overlay" hidden>
    <video id="call-remote" autoplay playsinline></video>
    <div id="call-top">
        <div id="call-peer"></div>
        <div id="call-status"></div>
    </div>
    <video id="call-local" autoplay playsinline muted></video>
    <button id="call-tap-audio" hidden>🔊 Tap to hear</button>
    <div id="call-controls">
        <button id="call-mute" title="Mute microphone" aria-label="Mute microphone"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 14a3 3 0 0 0 3-3V5a3 3 0 0 0-6 0v6a3 3 0 0 0 3 3zm5-3a5 5 0 0 1-10 0H5a7 7 0 0 0 6 6.92V21h2v-3.08A7 7 0 0 0 19 11h-2z"/><line class="slash" x1="3" y1="3" x2="21" y2="21"/></svg></button>
        <button id="call-cam" title="Turn camera off" aria-label="Turn camera off"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M17 10.5V7a1 1 0 0 0-1-1H4a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-3.5l4 4v-11l-4 4z"/><line class="slash" x1="3" y1="3" x2="21" y2="21"/></svg></button>
        <button id="call-flip" title="Switch camera" aria-label="Switch camera" hidden><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 6V3L8 7l4 4V8a4 4 0 0 1 4 4h2a6 6 0 0 0-6-6zm-4 6H6a6 6 0 0 0 6 6v3l4-4-4-4v3a4 4 0 0 1-4-4z"/></svg></button>
        <button id="call-hangup" class="hangup" title="Hang up" aria-label="Hang up"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 9c-1.6 0-3.15.25-4.6.72v3.1c0 .39-.23.74-.56.9-.98.49-1.87 1.12-2.66 1.85-.18.18-.43.28-.7.28-.28 0-.53-.11-.71-.29L.29 13.08a1 1 0 0 1 0-1.41C3.34 8.78 7.46 7 12 7s8.66 1.78 11.71 4.67c.18.18.29.43.29.71 0 .28-.11.53-.29.71l-2.48 2.48c-.18.18-.43.29-.71.29-.27 0-.52-.11-.7-.28a11.3 11.3 0 0 0-2.67-1.85 1 1 0 0 1-.56-.9v-3.1C15.15 9.25 13.6 9 12 9z"/></svg></button>
    </div>
</div>
`;

window.bfCall = (function () {
    const $ = (id) => document.getElementById(id);
    if (!$('call-overlay')) document.body.insertAdjacentHTML('beforeend', OVERLAY_HTML);
    const overlay = $('call-overlay');
    const remoteVideo = $('call-remote');
    const localVideo = $('call-local');
    const tapAudio = $('call-tap-audio');
    const muteBtn = $('call-mute');
    const camBtn = $('call-cam');
    const flipBtn = $('call-flip');

    const events = [];
    let s = null; // the current call session

    function emit(e) { events.push(e); }
    function send(sess, msg) {
        emit({ kind: 'signal', call_id: sess.callId, data: JSON.stringify(msg) });
    }
    function setStatus(text) { $('call-status').textContent = text; }
    function stopTracks(stream) {
        if (stream) stream.getTracks().forEach((t) => t.stop());
    }

    function fail(sess, message) {
        if (s !== sess) return;
        end();
        emit({ kind: 'error', call_id: sess.callId, message });
    }

    function videoConstraints(facing) {
        return { facingMode: facing, width: { ideal: 1280 }, height: { ideal: 720 } };
    }

    async function start(callId, role, iceJson, peer) {
        end();
        let iceServers = [];
        try { iceServers = JSON.parse(iceJson); } catch (_) {}
        const sess = s = {
            callId, role, peer, pc: null, local: null, ready: false,
            pending: [], iceQueue: [], chain: Promise.resolve(),
            facing: 'user', restarted: false, timer: null,
        };

        $('call-peer').textContent = peer;
        setStatus('Connecting…');
        tapAudio.hidden = true;
        muteBtn.classList.remove('toggled');
        camBtn.classList.remove('toggled');
        flipBtn.hidden = true;
        localVideo.className = 'mirror';
        overlay.hidden = false;

        if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
            fail(sess, 'Video calls need a secure (https) page.');
            return;
        }
        const audio = { echoCancellation: true, noiseSuppression: true, autoGainControl: true };
        try {
            sess.local = await navigator.mediaDevices.getUserMedia({ audio, video: videoConstraints('user') });
        } catch (err) {
            if (err && err.name === 'NotAllowedError') {
                fail(sess, 'Camera/microphone permission was denied.');
                return;
            }
            // No camera (or it's busy) — carry on with voice only
            try {
                sess.local = await navigator.mediaDevices.getUserMedia({ audio });
            } catch (_) {
                fail(sess, 'Could not access your microphone.');
                return;
            }
        }
        if (s !== sess) { stopTracks(sess.local); return; } // hung up while asking permission

        localVideo.srcObject = sess.local;
        if (sess.local.getVideoTracks().length === 0) {
            localVideo.classList.add('off');
            camBtn.classList.add('toggled');
        }
        navigator.mediaDevices.enumerateDevices().then((devices) => {
            if (s === sess && devices.filter((d) => d.kind === 'videoinput').length > 1) {
                flipBtn.hidden = false;
            }
        }).catch(() => {});

        const pc = sess.pc = new RTCPeerConnection({ iceServers });
        sess.local.getTracks().forEach((t) => pc.addTrack(t, sess.local));
        pc.onicecandidate = (e) => { if (e.candidate) send(sess, { candidate: e.candidate.toJSON() }); };
        pc.ontrack = (e) => {
            if (remoteVideo.srcObject !== e.streams[0]) {
                remoteVideo.srcObject = e.streams[0];
                remoteVideo.play().catch(() => { tapAudio.hidden = false; });
            }
        };
        pc.onconnectionstatechange = () => {
            if (s !== sess) return;
            const state = pc.connectionState;
            if (state === 'connected') {
                startTimer(sess);
            } else if (state === 'disconnected') {
                setStatus('Reconnecting…');
            } else if (state === 'failed') {
                // One ICE restart (caller drives it) before giving up
                if (sess.role === 'caller' && !sess.restarted) {
                    sess.restarted = true;
                    setStatus('Reconnecting…');
                    sess.chain = sess.chain.then(() => makeOffer(sess, true)).catch(() => {});
                } else {
                    fail(sess, "Couldn't connect the call — the network may be blocking it.");
                }
            }
        };

        sess.ready = true;
        if (role === 'caller') {
            sess.chain = sess.chain.then(() => makeOffer(sess, false));
        }
        sess.pending.splice(0).forEach((msg) => queue(sess, msg));
        sess.chain.catch((err) => fail(sess, 'Call setup failed: ' + err));
    }

    async function makeOffer(sess, iceRestart) {
        const offer = await sess.pc.createOffer({ iceRestart });
        await sess.pc.setLocalDescription(offer);
        send(sess, { sdp: sess.pc.localDescription });
    }

    async function handle(sess, msg) {
        const pc = sess.pc;
        if (msg.sdp) {
            await pc.setRemoteDescription(msg.sdp);
            if (msg.sdp.type === 'offer') {
                await pc.setLocalDescription(await pc.createAnswer());
                send(sess, { sdp: pc.localDescription });
            }
            for (const c of sess.iceQueue.splice(0)) {
                await pc.addIceCandidate(c).catch(() => {});
            }
        } else if (msg.candidate) {
            if (pc.remoteDescription) {
                await pc.addIceCandidate(msg.candidate).catch(() => {});
            } else {
                sess.iceQueue.push(msg.candidate);
            }
        }
    }

    // Signaling must be applied in order, one at a time
    function queue(sess, msg) {
        sess.chain = sess.chain
            .then(() => handle(sess, msg))
            .catch((err) => fail(sess, 'Call setup failed: ' + err));
    }

    function signal(callId, data) {
        if (!s || s.callId !== callId) return;
        let msg;
        try { msg = JSON.parse(data); } catch (_) { return; }
        if (s.ready) queue(s, msg); else s.pending.push(msg);
    }

    function startTimer(sess) {
        if (sess.timer) { setStatus(''); return; }
        const began = Date.now();
        const tick = () => {
            const secs = Math.floor((Date.now() - began) / 1000);
            setStatus(Math.floor(secs / 60) + ':' + String(secs % 60).padStart(2, '0'));
        };
        tick();
        sess.timer = setInterval(tick, 1000);
    }

    function end() {
        ring('');
        if (!s) return;
        const sess = s;
        s = null;
        clearInterval(sess.timer);
        stopTracks(sess.local);
        if (sess.pc) sess.pc.close();
        remoteVideo.srcObject = null;
        localVideo.srcObject = null;
        overlay.hidden = true;
    }

    function takeEvents() {
        return JSON.stringify(events.splice(0));
    }

    // ── Controls ────────────────────────────────────────────
    $('call-hangup').addEventListener('click', () => {
        if (!s) return;
        const callId = s.callId;
        end();
        emit({ kind: 'hangup', call_id: callId });
    });

    muteBtn.addEventListener('click', () => {
        if (!s || !s.local) return;
        const track = s.local.getAudioTracks()[0];
        if (!track) return;
        track.enabled = !track.enabled;
        muteBtn.classList.toggle('toggled', !track.enabled);
    });

    camBtn.addEventListener('click', () => {
        if (!s || !s.local) return;
        const track = s.local.getVideoTracks()[0];
        if (!track) return;
        track.enabled = !track.enabled;
        camBtn.classList.toggle('toggled', !track.enabled);
        localVideo.classList.toggle('off', !track.enabled);
    });

    flipBtn.addEventListener('click', async () => {
        const sess = s;
        if (!sess || !sess.pc || !sess.local) return;
        const facing = sess.facing === 'user' ? 'environment' : 'user';
        const old = sess.local.getVideoTracks()[0];
        // Some phones can't open two cameras at once — release the old one first
        if (old) old.stop();
        try {
            const fresh = await navigator.mediaDevices.getUserMedia({ video: videoConstraints(facing) });
            const track = fresh.getVideoTracks()[0];
            if (s !== sess) { track.stop(); return; }
            const sender = sess.pc.getSenders().find((x) => x.track === old || (x.track && x.track.kind === 'video'));
            if (sender) await sender.replaceTrack(track);
            if (old) sess.local.removeTrack(old);
            sess.local.addTrack(track);
            track.enabled = !camBtn.classList.contains('toggled');
            sess.facing = facing;
            localVideo.srcObject = sess.local;
            localVideo.classList.toggle('mirror', facing === 'user');
        } catch (_) {
            setStatus("Couldn't switch camera");
        }
    });

    tapAudio.addEventListener('click', () => {
        remoteVideo.play().then(() => { tapAudio.hidden = true; }).catch(() => {});
    });

    // ── Ring tones (synthesized, no assets) ─────────────────
    let audioCtx = null;
    let ringTimer = null;

    // Two-tone 440+480 Hz, like a North American phone line
    function tone(duration) {
        const t0 = audioCtx.currentTime;
        const gain = audioCtx.createGain();
        gain.gain.setValueAtTime(0, t0);
        gain.gain.linearRampToValueAtTime(0.12, t0 + 0.02);
        gain.gain.setValueAtTime(0.12, t0 + duration - 0.02);
        gain.gain.linearRampToValueAtTime(0, t0 + duration);
        gain.connect(audioCtx.destination);
        [440, 480].forEach((freq) => {
            const osc = audioCtx.createOscillator();
            osc.frequency.value = freq;
            osc.connect(gain);
            osc.start(t0);
            osc.stop(t0 + duration);
        });
    }

    function ring(kind) {
        clearInterval(ringTimer);
        clearTimeout(ringTimer);
        ringTimer = null;
        if (!kind) return;
        try {
            audioCtx = audioCtx || new (window.AudioContext || window.webkitAudioContext)();
            audioCtx.resume();
        } catch (_) { return; }
        if (kind === 'incoming') {
            // ring-ring … pause
            const burst = () => {
                tone(0.4);
                setTimeout(() => { if (ringTimer) tone(0.4); }, 600);
                if (navigator.vibrate) navigator.vibrate([400, 200, 400]);
            };
            burst();
            ringTimer = setInterval(burst, 3000);
        } else {
            // ringback: 2s on, 4s off
            tone(2);
            ringTimer = setInterval(() => tone(2), 6000);
        }
    }

    return { start, signal, end, ring, takeEvents };
})();

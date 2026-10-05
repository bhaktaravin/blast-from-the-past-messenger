# Self-host kit — operator guide

This document is for **you** (or your customers) running Blast From The Past on **your own** infrastructure. It states **what this stack is for**, what it is **not**, and how to run it reliably.

---

## Positioning (read this before comparing to WhatsApp or Discord)

| | Mass-market apps (WhatsApp, Discord, …) | This project |
|--|--|--|
| **Goal** | Billions of users, one global network | **Your** server, **your** community |
| **Moat** | Network effects (everyone is already there) | **Privacy, control, nostalgia UX**, self-hosting |
| **Mobile** | First-class native apps, push, background | **Web (WASM)** works in the browser; desktop is a separate native build |
| **Ops** | Huge SRE teams | **You** operate Postgres, Redis, TLS, backups, abuse |

**Honest summary:** This is **not** a drop-in “we will beat Discord” consumer product on day one. It **is** a strong **self-hosted retro messenger** for teams, friend groups, events, or niche communities that want **control** and a **deliberately nostalgic** experience.

Sell and describe it that way; buyers who want Discord-scale moderation, mobile parity, and network effects will be disappointed unless you invest heavily in those areas later.

---

## What’s included in the kit

- **Server** (`server` binary): WebSocket chat backend, requires **PostgreSQL** + **Redis**.
- **Clients**:
  - **Native desktop** (Rust + egui), optional features like desktop notifications.
  - **Web client** (WASM via Trunk): primary path for “anyone with a browser.”
- **Features** (see README): lobby, rooms, DMs, friends, themes, optional **E2E for DMs** (see Security notes), etc.
- **Video (web):** Uses **Jitsi** in the browser; see `VIDEO_CALLING_*.md` for behavior and limits.

---

## Requirements

- **Rust** toolchain (for building from source).
- **PostgreSQL** and **Redis** (managed or self-run).
- **TLS termination** for production WebSockets: browsers need **`wss://`**, not raw `ws://` on the public internet (use a reverse proxy or your host’s HTTPS).

---

## Quick start — local Docker (recommended for first boot)

From the repo root, with Docker:

```bash
docker compose up --build
```

Defaults (development-style — **change before exposing to the internet**):

- Postgres: `retro` / `retro` / DB `retrochat` (see `docker-compose.yml`).
- Server WebSocket: `ws://localhost:9001` (use `ws://` only on localhost).

### Native client → local server

Run the client with the client feature (see README). On the **sign-in** screen, set **Server URL** to:

`ws://localhost:9001`

(or `127.0.0.1:9001` — the client normalizes the scheme.)

### Web client → local server

Build/serve with Trunk (`trunk serve`). Ensure the **Server URL** field on the login screen points at a URL your **browser** can reach (often `ws://127.0.0.1:9001` for dev). Production web must use **`wss://`** behind HTTPS.

---

## Production checklist (minimal)

1. **Secrets:** Strong DB password, rotate Redis if password-protected, no default compose credentials on the public internet.
2. **`BIND_ADDR`:** e.g. `0.0.0.0:9001` behind a reverse proxy, or whatever your host expects (e.g. Railway + `$PORT` — see `railway.toml` / host docs).
3. **`DATABASE_URL` / `REDIS_URL`:** Point at production secrets (env vars, not committed files).
4. **HTTPS + `wss://`:** Terminate TLS at Caddy, nginx, Traefik, or your PaaS so the web app uses secure WebSockets.
5. **CORS / origin:** If the web UI is on a different host than the API, confirm your server’s CORS/WebSocket origin policy matches (adjust server code if needed for your domain).
6. **Backups:** Postgres backups on a schedule you can restore from.

---

## Server URL — what customers actually connect to

The **sign-in screen** has a **Server URL** field. Users can paste your public WebSocket endpoint there (typically `wss://chatapi.yourdomain.com` or your PaaS URL).

The **default** value baked into the client source is a **demo** deployment. For **your** product:

- **Option A (no rebuild):** Tell users to replace **Server URL** with **your** `wss://…` endpoint before signing on.
- **Option B (white-label build):** Change the default in `src/main.rs` (the initial `server_url` in `App` state) and rebuild WASM/native so the field pre-fills with **your** server.

---

## Security notes (keep marketing accurate)

- **Authentication:** Passwords are handled server-side with normal app semantics; use TLS everywhere in production.
- **E2E DMs:** When enabled, **direct messages** can use the project’s E2E design; **the server still sees metadata** (who talks to whom, timing, presence, etc.). **Lobby and rooms** are not magically E2E unless you implement and document otherwise.
- **Jitsi:** Video runs in the browser through **third-party** infrastructure; factor that into privacy promises for strict buyers.

---

## Support boundary (suggested text for your Gumroad page)

Define what you will help with (e.g. “following this guide on Docker/Railway”) vs out of scope (custom features, rewriting for SOC2, guaranteeing Discord parity). Reduces refund friction and sets expectations.

---

## Suggested Gumroad “what you get” bullets

- Full **source code** and **build instructions** (Rust).
- **Docker Compose** path for Postgres + Redis + server.
- Pointers for **web (WASM)** and **desktop** builds.
- Clear list of **features** and **non-goals** (self-hosted niche messenger, not a global social network).

---

## Packaging the kit (for Gumroad / email)

From the repo root:

```bash
./scripts/package-kit.sh
```

Creates a timestamped zip on your Desktop (override with `BFTP_OUTPUT_DIR=/path`). Uses `git archive` so the archive matches the last commit — **commit your changes first**.

## Related docs in this repo

- `README.md` — feature overview and run commands.
- `DEPLOYMENT_CHECKLIST.md` — Amplify + Railway alignment.
- `AMPLIFY_SETUP.md` / `AWS_WEB_DEPLOYMENT.md` — static web hosting.
- `VIDEO_CALLING_STATUS.md` — video calling scope (web vs native).

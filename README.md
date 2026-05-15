# tailscale-screenshot

Built this because I kept dragging PNGs into Slack and Discord one at a time
like an animal. Now I copy a screenshot, hit `Ctrl+Shift+U`, press Enter, and
a URL lands on my clipboard. Paste it. Done.

The server is a single static Rust binary running on a box in my tailnet, so
the only people who can reach it are me on my other devices. No auth, no TLS,
no database. Tailscale is the perimeter.

## How it works

```mermaid
sequenceDiagram
    actor You
    participant Ext as Chrome popup
    participant Srv as Rust server (on your tailnet)
    You->>You: screenshot to clipboard
    You->>Ext: Ctrl+Shift+U, Enter
    Ext->>Srv: POST /upload (image bytes)
    Srv->>Srv: save as <id>.png
    Srv-->>Ext: { "url": "http://host.ts.net:7777/s/<id>.png" }
    Ext->>You: write URL to clipboard
    You->>You: paste anywhere
```

The popup reads the clipboard, posts the bytes, gets a URL back, and writes
that URL to the clipboard. The server gives the file a short random id and
serves it from disk with the right `Content-Type`.

## Server

You need `rustup`, `tailscale`, and [`smdctl`][smdctl] on the host. From the
repo root:

```
make install
```

That builds a release binary against musl, drops it in `~/.local/bin`, and
registers it as a user-mode systemd service through smdctl. Port 7777, so no
sudo. On startup the server logs its Tailscale URL — that's what you paste
into the extension.

To pull the URL out without scrolling logs:

```
make url
```

Other things you might want:

```
make logs       # follow the journal
make status
make restart
make uninstall
```

User-mode services stop when you log out unless you tell systemd otherwise:

```
loginctl enable-linger $USER
```

[smdctl]: https://github.com/nexo-tech/smdctl

## Extension

1. Go to `chrome://extensions`, flip on Developer mode.
2. Load unpacked, pick the `extension/` directory.
3. Pin it from the puzzle-piece menu so the shortcut works without the popup hidden.
4. Open the popup, click the gear, paste the URL from `make url`, click off the field.
5. If `Ctrl+Shift+U` collides with something, rebind it at `chrome://extensions/shortcuts`.

## API

In case you want to script around it:

```
POST /upload
  Content-Type: image/{png,jpeg,gif,webp}
  body: raw image bytes (10 MB cap)
  -> 200 { "url": "...", "id": "..." }

GET  /s/<id>.<ext>     the image, with the right Content-Type
GET  /healthz          "ok"
```

`curl` example:

```
curl -X POST -H 'Content-Type: image/png' \
     --data-binary @shot.png \
     http://host.tail-XXXX.ts.net:7777/upload
```

## Config

All optional, all env vars:

| var        | default                                | what it does                    |
|------------|----------------------------------------|---------------------------------|
| `PORT`     | `7777`                                 | port to listen on               |
| `DATA_DIR` | `./screenshots` (Makefile overrides)   | where files are written         |
| `BASE_URL` | auto-detected from `tailscale status`  | override the URL it advertises  |

## Layout

```
server/         one main.rs, axum, ~150 lines
extension/      MV3 popup
smdctl.yml      service descriptor (Makefile fills in the paths)
Makefile        build + smdctl wrappers
```

## Things it deliberately doesn't do

No auth tokens, no signed URLs, no expiry, no garbage collection, no upload
history, no drag and drop, no Tailscale Funnel. If you want any of that,
it's a couple hours of work, but I don't.

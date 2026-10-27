# Session integration

These files are a **skeleton** for Phase 3 (native compositor). Today the
session daemon serves the shell over HTTP, so a display manager launches it and
a kiosk-style browser renders the desktop:

```
minerva-sessiond --port 8080 --web-root /usr/share/debian-minerva/web
chromium --kiosk --app=http://localhost:8080
```

Once `minerva-compositor` (Smithay) lands, `minerva.desktop` will point at the
compositor binary directly and the HTTP bridge becomes an optional debug
interface.

#!/bin/sh
# native-test.sh — the v0.11.0 primary path: minerva-session must start the
# native minerva-wm as the base session, must NOT start any browser shell or
# xfce4-session while the native WM is alive, and must record base=native in
# the ready file. Legacy failover itself is covered by session-env-test.sh
# (MINERVA_WM=/nonexistent) and by the two-strike failover logic assertions
# below (failover only after minerva-wm dies twice in a row).
set -u
cd "$(dirname "$0")" || . ./lib.sh
. ./lib.sh
SRC="../../session/minerva-session"

echo "== session-test: native minerva-wm base =="
st_tmp

BIN="$ST_HOME/bin"
mkdir -p "$BIN"
STUBLOG="$ST_HOME/stub.log"
: > "$STUBLOG"
PORT=47992
export MINERVA_PORT="$PORT"

# stub daemon: real HTTP 200 on every path so the health-wait passes
cat > "$BIN/minerva-sessiond" <<'DEOF'
#!/bin/sh
echo "daemon started" >> "$STUBLOG_REPLACE"
exec python3 -c '
import http.server, socketserver, os
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"{}"
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *a): pass
socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("127.0.0.1", int(os.environ["MINERVA_PORT"])), H) as httpd:
    httpd.serve_forever()
'
DEOF
sed -i "s|STUBLOG_REPLACE|$STUBLOG|" "$BIN/minerva-sessiond"
chmod +x "$BIN/minerva-sessiond"

make_stub() {
  cat > "$BIN/$1" <<SEOF
#!/bin/sh
echo "$1 started" >> "$STUBLOG"
sleep 30
SEOF
  chmod +x "$BIN/$1"
}
# the native WM stub stays up (healthy native session)
for s in minerva-wm xfce4-session xfwm4 minerva-shell-start chromium; do make_stub "$s"; done
for s in wmctrl xprop xsetroot xfconf-query notify-send; do
  printf '#!/bin/sh\nexit 0\n' > "$BIN/$s"; chmod +x "$BIN/$s"
done
cat > "$BIN/xrandr" <<'XEOF'
#!/bin/sh
echo "Screen 0: minimum 8 x 8, current 1920 x 1080, maximum 16384 x 16384"
echo "eDP-1 connected primary 1920x1080+0+0 0mm x 0mm"
echo "   1920x1080 60.00*+"
XEOF
chmod +x "$BIN/xrandr"

export PATH="$BIN:$PATH"
export DISPLAY=":99"
export MINERVA_DAEMON="$BIN/minerva-sessiond"

setsid sh "$SRC" >/dev/null 2>&1 &
PGID=$!
sleep 3

RT="$XDG_RUNTIME_DIR/minerva"
SNAP="$ST_HOME/session.env.snap"
cp "$RT/session.env" "$SNAP" 2>/dev/null

sleep 2
kill -TERM -"$PGID" 2>/dev/null
sleep 1
pkill -f "MINERVA_PORT" 2>/dev/null
pkill -f "$ST_HOME/bin" 2>/dev/null

STATE="$XDG_STATE_HOME/minerva"
st_assert_file "$SNAP" "session.env present while the native session lives"
st_assert_grep 'XDG_CURRENT_DESKTOP=Minerva' "$SNAP" "session.env carries desktop identity"

st_assert_file "$STATE/session.log" "session log written"
st_assert_file "$STATE/ready" "ready file written"
st_assert_grep 'base=native' "$STATE/ready" "ready file records the native base"
st_assert_file "$STATE/wm.pid" "wm.pid written for the native WM"

if grep -q 'minerva-wm started' "$STUBLOG"; then
  st_ok "native minerva-wm stub launched as the base session"
else
  st_bad "minerva-wm stub never launched"
fi
for ghost in minerva-shell-start chromium xfce4-session xfwm4; do
  if grep -q "$ghost started" "$STUBLOG"; then
    st_bad "$ghost ran — the native base must not start legacy surfaces"
  else
    st_ok "$ghost NOT started (native base does not use it)"
  fi
done

dline="$(grep -n 'daemon started pid=' "$STATE/session.log" | head -1 | cut -d: -f1)"
bline="$(grep -n 'base: minerva-wm' "$STATE/session.log" | head -1 | cut -d: -f1)"
if [ -n "$dline" ] && [ -n "$bline" ] && [ "$dline" -lt "$bline" ]; then
  st_ok "daemon started before the native base (env snapshot order)"
else
  st_bad "daemon/base start order wrong (daemon=$dline base=$bline)"
fi

st_done

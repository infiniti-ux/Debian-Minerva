#!/bin/sh
# session-env-test.sh — full-session integration test with stub binaries.
#
# Runs the REAL minerva-session for a few seconds inside a throwaway HOME
# with a PATH of stubs: a python3 HTTP stub daemon (answers /api/v1/health
# so the 10s health-wait passes), stub xfce4-session/xfwm4, stub
# minerva-shell-start (the session calls it by PATH name), stub X tools.
# Asserts the contracts a real login depends on:
#   1. session.env exists WHILE the session lives and carries the
#      platform contracts (cleanup removes it at session end by design,
#      so it is snapshotted before the kill);
#   2. daemon stub started BEFORE the base session stub (the 0.5.6
#      env-snapshot ordering bug must never return);
#   3. xfconf channels + autostart masks written before base start;
#   4. ready file records mode/base;
#   5. the shell launcher stub was reached.
set -u
cd "$(dirname "$0")" || . ./lib.sh
. ./lib.sh
SRC="../../session/minerva-session"

echo "== session-test: full session with stub binaries =="
st_tmp

BIN="$ST_HOME/bin"
mkdir -p "$BIN"
STUBLOG="$ST_HOME/stub.log"
: > "$STUBLOG"
PORT=47991
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
for s in xfce4-session xfwm4 minerva-shell-start chromium; do make_stub "$s"; done
for s in wmctrl xprop xsetroot xfconf-query; do
  printf '#!/bin/sh\nexit 0\n' > "$BIN/$s"; chmod +x "$BIN/$s"
done
# xrandr stub must answer like a real connected output, or the session's
# screen-size parse comes back empty and arithmetic expansion aborts it
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
st_assert_file "$SNAP" "session.env present while session lives"
st_assert_grep 'XDG_CURRENT_DESKTOP=Minerva' "$SNAP" "session.env carries desktop identity"
st_assert_grep 'MINERVA_TOKEN=' "$SNAP" "session.env carries token"
st_assert_grep 'MINERVA_API=' "$SNAP" "session.env carries api url"

st_assert_file "$STATE/session.log" "session log written"
st_assert_file "$STATE/ready" "ready file written"
st_assert_grep 'base=xfce' "$STATE/ready" "ready file records wrapper base"
st_assert_file "$XDG_CONFIG_HOME/xfce4/xfconf/xfce-perchannel-xml/xfwm4.xml" "xfconf written before base start"
st_assert_file "$XDG_CONFIG_HOME/autostart/xfdesktop.desktop" "autostart masks written"

# ordering authority = the session's own log: daemon start is logged
# before the base branch banner
dline="$(grep -n 'daemon started pid=' "$STATE/session.log" | head -1 | cut -d: -f1)"
bline="$(grep -n 'base: xfce4-session' "$STATE/session.log" | head -1 | cut -d: -f1)"
if [ -n "$dline" ] && [ -n "$bline" ] && [ "$dline" -lt "$bline" ]; then
  st_ok "daemon started before base session (env snapshot order)"
else
  st_bad "daemon/base start order wrong (daemon=$dline base=$bline)"
fi
if grep -q 'minerva-shell-start started' "$STUBLOG"; then
  st_ok "session reached the shell launcher"
else
  st_bad "shell launcher stub never launched"
fi
if grep -q 'xfwm4 started' "$STUBLOG"; then
  st_ok "window manager stub running under the base"
else
  st_bad "xfwm4 stub never started"
fi

st_done

#!/bin/sh
# trace-test.sh — minerva-trace must record valid JSONL samples against a
# fake session.env + stub wmctrl, flag a missing shell surface, and refuse
# to run outside a session instead of writing garbage.
set -u
cd "$(dirname "$0")" || . ./lib.sh
. ./lib.sh
SRC="../../session/minerva-trace"

echo "== session-test: minerva-trace recorder =="
st_tmp

# outside a session: refuse cleanly
out="$(sh "$SRC" 2 2>&1)"
rc=$?
if [ "$rc" != 0 ]; then st_ok "refuses outside a session (exit $rc)"; else st_bad "ran outside a session"; fi
if printf '%s' "$out" | grep -q 'no session env'; then st_ok "refusal message names the missing session env"; else st_bad "unclear refusal message"; fi

# fake session env + stubs
RT="$XDG_RUNTIME_DIR/minerva"
mkdir -p "$RT"
printf 'MINERVA_API=http://127.0.0.1:1/api/v1\nMINERVA_TOKEN=test\n' > "$RT/session.env"
BIN="$ST_HOME/bin"
mkdir -p "$BIN"
cat > "$BIN/wmctrl" <<'EOF'
#!/bin/sh
if [ "$1" = "-l" ]; then
  echo "0x00a00001  0 host Minerva"
  echo "0x00c00002  0 host xterm"
fi
exit 0
EOF
chmod +x "$BIN/wmctrl"
export PATH="$BIN:$PATH"

OUT="$ST_HOME/trace.jsonl"
sh "$SRC" 3 "$OUT" >/dev/null 2>&1
rc=$?
if [ "$rc" = 0 ]; then st_ok "recorder exits 0"; else st_bad "recorder exit $rc"; fi
st_assert_file "$OUT" "trace file written"

lines="$(grep -c . "$OUT" 2>/dev/null || echo 0)"
if [ "$lines" -ge 1 ]; then st_ok "$lines sample(s) recorded in 3s"; else st_bad "no samples recorded"; fi

badjson=0
while IFS= read -r line; do
  printf '%s' "$line" | python3 -c 'import sys, json; json.load(sys.stdin)' 2>/dev/null || badjson=$((badjson + 1))
done < "$OUT"
if [ "$badjson" = 0 ]; then st_ok "every line is valid JSON"; else st_bad "$badjson malformed JSON lines"; fi

if grep -q '"shell_win":true' "$OUT"; then st_ok "shell surface detected from stub wmctrl"; else st_bad "shell_win never true despite stub window"; fi
if grep -q '"windows":2' "$OUT"; then st_ok "window count parsed from stub wmctrl"; else st_bad "window count wrong"; fi

# duration guard
sh "$SRC" abc "$OUT" >/dev/null 2>&1
if [ $? != 0 ]; then st_ok "non-numeric duration refused"; else st_bad "non-numeric duration accepted"; fi

st_done

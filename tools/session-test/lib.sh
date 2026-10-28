# tools/session-test/lib.sh — shared helpers for the session-script battery.
# Tests extract individual functions from the real session scripts and run
# them against throwaway HOME/XDG trees, so a broken session function fails
# the build instead of failing the user's login.

st_fail=0

st_ok()   { printf '  ok    %s\n' "$1"; }
st_bad()  { printf '  FAIL  %s\n' "$1"; st_fail=$((st_fail + 1)); }

st_tmp() {
  ST_HOME="$(mktemp -d)"
  export HOME="$ST_HOME"
  export XDG_CONFIG_HOME="$ST_HOME/.config"
  export XDG_STATE_HOME="$ST_HOME/.local/state"
  export XDG_RUNTIME_DIR="$ST_HOME/run"
  mkdir -p "$XDG_CONFIG_HOME" "$XDG_STATE_HOME" "$XDG_RUNTIME_DIR"
}

st_cleanup() { [ -n "${ST_HOME:-}" ] && rm -rf "$ST_HOME"; }

# st_extract <script> <function-name> — print the function body verbatim.
st_extract() {
  awk -v fn="$2" '
    $0 == fn "() {" { inside = 1 }
    inside { print }
    inside && $0 == "}" { exit }
  ' "$1"
}

# st_assert_file <path> <description>
st_assert_file() {
  if [ -f "$1" ]; then st_ok "$2"; else st_bad "$2 (missing: $1)"; fi
}

# st_assert_grep <pattern> <file> <description>
st_assert_grep() {
  if grep -q "$1" "$2" 2>/dev/null; then st_ok "$3"; else st_bad "$3 (pattern '$1' not in $2)"; fi
}

# st_assert_xml <file> — well-formed XML via python3
st_assert_xml() {
  if python3 -c "import sys, xml.etree.ElementTree as ET; ET.parse(sys.argv[1])" "$1" 2>/dev/null; then
    st_ok "xml well-formed: $(basename "$1")"
  else
    st_bad "xml malformed: $1"
  fi
}

st_done() {
  st_cleanup
  if [ "$st_fail" = 0 ]; then echo "  result: PASSED"; else echo "  result: $st_fail FAILURE(S)"; fi
  [ "$st_fail" = 0 ]
}

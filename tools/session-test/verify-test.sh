#!/bin/sh
# verify-test.sh — minerva-verify must produce the one-paste evidence
# bundle anywhere: report file created in HOME, both sub-reports inside,
# and the contract env section present when the contracts are exported.
set -u
cd "$(dirname "$0")" || . ./lib.sh
. ./lib.sh
SRC="../../session/minerva-verify"

echo "== session-test: minerva-verify bundle =="
st_tmp
export PATH="$(cd ../../session && pwd):$PATH"
export XDG_CURRENT_DESKTOP=Minerva

sh "$SRC" >/dev/null 2>&1
rc=$?
if [ "$rc" = 0 ]; then st_ok "verify exits 0 on a dev machine"; else st_bad "verify exit $rc"; fi

REP="$HOME/minerva-report.txt"
st_assert_file "$REP" "report file written to HOME"
st_assert_grep '===== minerva-verify' "$REP" "report has header stamp"
st_assert_grep 'env: XDG_CURRENT_DESKTOP=Minerva' "$REP" "contract env dumped into report"
st_assert_grep 'Minerva bootstrap' "$REP" "bootstrap matrix section inside"
st_assert_grep 'Minerva Doctor' "$REP" "doctor section inside"

# second run overwrites cleanly (no append garbage)
sh "$SRC" >/dev/null 2>&1
n="$(grep -c '===== minerva-verify' "$REP")"
if [ "$n" = 1 ]; then st_ok "report is replaced, not appended"; else st_bad "report has $n headers (append bug)"; fi

st_done

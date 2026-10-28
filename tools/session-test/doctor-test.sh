#!/bin/sh
# doctor-test.sh — minerva-doctor must run clean outside a session (warns,
# not crashes), print every report section including the 0.9.0 platform
# contracts block, and keep its exit code inside the documented 0..9 range.
set -u
cd "$(dirname "$0")" || . ./lib.sh
. ./lib.sh
SRC="../../session/minerva-doctor"

echo "== session-test: minerva-doctor outside a session =="
st_tmp
out="$(sh "$SRC" 2>&1)"
rc=$?
if [ "$rc" -le 9 ]; then st_ok "exit code $rc within documented 0..9"; else st_bad "exit code $rc out of range"; fi

for sect in '[session]' '[daemon]' '[base session]' '[shell surface]' '[geometry]' '[browser + apps]' '[services]' '[platform contracts]' '[install consistency]' '[logs]'; do
  if printf '%s\n' "$out" | grep -qF "$sect"; then st_ok "section present: $sect"; else st_bad "section missing: $sect"; fi
done

if printf '%s\n' "$out" | grep -q 'XDG_CURRENT_DESKTOP='; then
  st_ok "platform-contract identity check ran"
else
  st_bad "platform-contract identity check absent"
fi
if printf '%s\n' "$out" | grep -qE '^(  OK|  FAIL|  WARN)  '; then
  st_ok "report uses OK/FAIL/WARN line format"
else
  st_bad "no OK/FAIL/WARN lines in report"
fi

st_done

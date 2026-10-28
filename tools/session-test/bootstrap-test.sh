#!/bin/sh
# bootstrap-test.sh — the component matrix must parse, print one OK/MISSING
# line per component, cover the platform-contract rows added in 0.9.0, and
# refuse --fix without root instead of half-installing things.
set -u
cd "$(dirname "$0")" || . ./lib.sh
. ./lib.sh
SRC="../../session/minerva-bootstrap"

echo "== session-test: minerva-bootstrap matrix =="
st_tmp
out="$(sh "$SRC" --check 2>&1)"
rc=$?
nmiss="$(printf '%s\n' "$out" | grep -c '^  MISSING  ')"
if [ "$nmiss" -gt 0 ]; then
  if [ "$rc" = 1 ]; then st_ok "--check exit 1 consistent with $nmiss MISSING row(s) (dev machine)"; else st_bad "--check exit $rc with $nmiss MISSING rows (must be 1)"; fi
else
  if [ "$rc" = 0 ]; then st_ok "--check exits 0 on a complete install"; else st_bad "--check exit $rc with no MISSING rows"; fi
fi

lines="$(printf '%s\n' "$out" | grep -c -E '^  (OK|MISSING)  ')"
if [ "$lines" -ge 24 ]; then st_ok "matrix prints $lines component rows (>=24)"; else st_bad "only $lines component rows"; fi

bad_fmt="$(printf '%s\n' "$out" | grep -E '^  (OK|MISSING)  ' | grep -c -v -E '^  (OK|MISSING)  [a-z0-9-]+ +[a-z0-9./-]+$')"
if [ "$bad_fmt" = 0 ]; then st_ok "every row matches name+package format"; else st_bad "$bad_fmt malformed rows"; fi

for row in portal-daemon portal-gtk-backend user-units user-preset mime-defaults auth-lock power-manager; do
  if printf '%s\n' "$out" | grep -q "  $row "; then st_ok "row present: $row"; else st_bad "row missing: $row"; fi
done

# --fix as non-root must not attempt apt
if [ "$(id -u)" != 0 ]; then
  fixout="$(sh "$SRC" --fix 2>&1)"
  if printf '%s' "$fixout" | grep -qi 'root'; then st_ok "--fix refuses without root"; else st_bad "--fix did not refuse without root"; fi
fi

st_done

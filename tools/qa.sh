#!/bin/sh
# tools/qa.sh — regression gate for the Minerva shell.
# Fails if any syntax check breaks or if a known bug pattern reappears.
set -u
cd "$(dirname "$0")/.." || exit 1
fail=0
for f in web/js/*.js; do node --check "$f" || fail=1; done
for f in session/minerva-* debian/postinst; do
  case "$f" in *.desktop|*.service) continue ;; esac
  [ -f "$f" ] && { dash -n "$f" || fail=1; }
done
# every session script must keep its executable bit (verify calls them by name)
for f in session/minerva-session session/minerva-shell-start session/minerva-doctor session/minerva-bootstrap session/minerva-verify session/minerva-trace session/minerva-dunstrc session/minerva-ctl session/minerva-autostart; do
  [ -x "$f" ] || { echo "qa: $f lost its executable bit"; fail=1; }
done

# bug patterns that must never come back
grep -n 'wait "\${SHELL_START_PID:-0}"' session/minerva-session && fail=1
grep -n 'animation: pop2 ' web/css/minerva.css && fail=1
grep -n "\$('#launcher-input').focus()" web/js/shell.js && fail=1
if [ "$fail" = 0 ]; then echo "qa: ALL GATES PASSED"; else echo "qa: GATES FAILED"; fi
exit "$fail"

#!/bin/sh
# tools/shell-test/run.sh — Node-side debug suite for the Minerva shell.
# Every check here runs offline in seconds and gates `make test-all`.
set -u
cd "$(dirname "$0")" || exit 1
fail=0

echo "== shell-test: syntax =="
for f in ../../web/js/*.js; do
  node --check "$f" || fail=1
done

echo "== shell-test: css integrity =="
node css-check.js ../../web/css/minerva.css || fail=1

echo "== shell-test: icon registry =="
node icons-test.js || fail=1

echo "== shell-test: app views render =="
node apps-view-test.js || fail=1

echo "== shell-test: cross-file static integrity =="
node static-check.js || fail=1

if [ "$fail" = 0 ]; then
  echo "shell-test: ALL PASSED"
else
  echo "shell-test: FAILURES PRESENT"
fi
exit "$fail"

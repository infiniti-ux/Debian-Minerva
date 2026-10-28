#!/bin/sh
# tools/session-test/run.sh — behavioural battery for the session scripts.
# Each test extracts a real function from the real script and runs it in a
# throwaway HOME, so regressions fail the build, not the user's login.
set -u
cd "$(dirname "$0")" || exit 1
fail=0
for t in xfconf-test.sh autostart-test.sh dunstrc-test.sh bootstrap-test.sh doctor-test.sh verify-test.sh trace-test.sh native-test.sh session-env-test.sh; do
  echo "--- $t"
  sh "$t" || fail=1
done
if [ "$fail" = 0 ]; then
  echo "session-test: ALL PASSED"
else
  echo "session-test: FAILURES PRESENT"
fi
exit "$fail"

#!/bin/sh
# dunstrc-test.sh — minerva-dunstrc must create the Aluminium dunstrc once,
# never clobber a user's own file, and stay silent/idempotent afterwards.
set -u
cd "$(dirname "$0")" || . ./lib.sh
. ./lib.sh
SRC="../../session/minerva-dunstrc"

echo "== session-test: minerva-dunstrc =="
st_tmp
DC="$XDG_CONFIG_HOME/dunst"

sh "$SRC"
st_assert_file "$DC/dunstrc" "dunstrc created on first run"
st_assert_grep '^\[global\]' "$DC/dunstrc" "dunstrc has [global] section"
st_assert_grep 'corner_radius = 12' "$DC/dunstrc" "Aluminium corner radius present"
st_assert_grep 'highlight = "#a80030"' "$DC/dunstrc" "Minerva accent highlight present"

cp "$DC/dunstrc" "$ST_HOME/first-run.copy"
echo "# my own settings" >> "$DC/dunstrc"
sh "$SRC"
if grep -q '# my own settings' "$DC/dunstrc"; then
  st_ok "user dunstrc never clobbered"
else
  st_bad "user dunstrc was overwritten by minerva-dunstrc"
fi

sh "$SRC"; sh "$SRC"
if cmp -s "$DC/dunstrc" "$DC/dunstrc"; then st_ok "repeat runs are no-ops"; else st_bad "repeat runs changed the file"; fi

st_done

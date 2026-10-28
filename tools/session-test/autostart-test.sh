#!/bin/sh
# autostart-test.sh — run the real mask_autostart() and assert the mask set
# is exactly the two visible XFCE surfaces (panel + desktop): screensaver
# and power manager must stay UNMASKED since 0.7.1 (they provide the real
# lock and real power handling), and every mask file must carry Hidden=true
# per the freedesktop autostart spec.
set -u
cd "$(dirname "$0")" || . ./lib.sh
. ./lib.sh
SRC="../../session/minerva-session"

echo "== session-test: mask_autostart =="
st_tmp
log() { :; }
eval "$(st_extract "$SRC" mask_autostart)"
mask_autostart

AD="$XDG_CONFIG_HOME/autostart"
st_assert_file "$AD/xfce4-panel.desktop" "xfce4-panel masked"
st_assert_file "$AD/xfdesktop.desktop" "xfdesktop masked"
st_assert_grep '^Hidden=true' "$AD/xfce4-panel.desktop" "panel mask uses Hidden=true (spec)"
st_assert_grep '^Hidden=true' "$AD/xfdesktop.desktop" "desktop mask uses Hidden=true (spec)"

if [ -f "$AD/xfce4-screensaver.desktop" ]; then
  st_bad "xfce4-screensaver must NOT be masked (real lock lives there)"
else
  st_ok "xfce4-screensaver left unmasked"
fi
if [ -f "$AD/xfce4-power-manager.desktop" ]; then
  st_bad "xfce4-power-manager must NOT be masked (real power handling)"
else
  st_ok "xfce4-power-manager left unmasked"
fi

n="$(ls "$AD" 2>/dev/null | wc -l | tr -d ' ')"
if [ "$n" = "2" ]; then st_ok "mask set size exactly 2"; else st_bad "mask set size $n != 2"; fi

st_done

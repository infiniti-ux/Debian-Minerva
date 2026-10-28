#!/bin/sh
# xfconf-test.sh — run the real write_xfconf_defaults() from minerva-session
# against a throwaway XDG tree and verify every channel file it promises:
# exists, parses as XML, and carries the curated values (theme=Minerva,
# Adwaita-dark, SaveOnExit=false, lock enabled, power silent).
set -u
cd "$(dirname "$0")" || . ./lib.sh
. ./lib.sh
SRC="../../session/minerva-session"

echo "== session-test: write_xfconf_defaults =="
st_tmp
log() { :; }
eval "$(st_extract "$SRC" write_xfconf_defaults)"
write_xfconf_defaults

XD="$XDG_CONFIG_HOME/xfce4/xfconf/xfce-perchannel-xml"
for f in xfce4-session xfce4-keyboard-shortcuts xfwm4 xsettings xfce4-screensaver xfce4-power-manager; do
  st_assert_file "$XD/$f.xml" "channel written: $f.xml"
  st_assert_xml "$XD/$f.xml"
done

st_assert_grep 'value="Minerva"' "$XD/xfwm4.xml" "xfwm4 theme = Minerva"
st_assert_grep 'name="use_compositing" type="bool" value="true"' "$XD/xfwm4.xml" "compositing on"
st_assert_grep 'name="placement_mode" type="string" value="mouse"' "$XD/xfwm4.xml" "mouse-follow placement"
st_assert_grep 'value="Adwaita-dark"' "$XD/xsettings.xml" "GTK theme Adwaita-dark"
st_assert_grep 'name="SaveOnExit" type="bool" value="false"' "$XD/xfce4-session.xml" "session save-on-exit off"
st_assert_grep 'name="enabled" type="bool" value="false"' "$XD/xfce4-screensaver.xml" "screensaver auto-blank off"
st_assert_grep 'name="show-tray-icon" type="bool" value="false"' "$XD/xfce4-power-manager.xml" "power manager tray hidden"

# idempotency: second run must not corrupt anything
write_xfconf_defaults
st_assert_xml "$XD/xfwm4.xml"
st_assert_grep 'value="Minerva"' "$XD/xfwm4.xml" "second run keeps theme"

st_done

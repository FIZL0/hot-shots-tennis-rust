#!/usr/bin/env bash
# Log the desktop in from a shell (SSH) when a reboot left SDDM at its password screen, so the runners, PCSX2 and
# wayvnc have a session: a one-shot SDDM autologin into Hyprland (uwsm), removed again once the session is up, so
# the next boot still asks for the password. Needs sudo. Then starts wayvnc in the new session (as you did by hand).
# ponytail: the session comes up unlocked on the physical screen; lock it from VNC if that matters
set -e
uid=$(id -u) f=/etc/sddm.conf.d/zz-hst-autologin.conf
sock() { compgen -G "/run/user/$uid/wayland-[0-9]*" | head -1; }
[ -n "$(sock)" ] && echo "desktop already up: $(sock)" || {
  printf '[Autologin]\nUser=%s\nSession=hyprland-uwsm.desktop\n' "$USER" | sudo tee "$f" >/dev/null
  trap 'sudo rm -f "$f"' EXIT
  sudo systemctl restart sddm
  for _ in $(seq 60); do [ -n "$(sock)" ] && break; sleep 1; done
  [ -n "$(sock)" ] || { echo "no session after 60 s; check: journalctl -b -u sddm | tail"; exit 1; }
  echo "logged in: $(sock)"
  sleep 3  # let Hyprland finish starting before wayvnc attaches
}
pgrep -u "$uid" -x wayvnc >/dev/null || WAYLAND_DISPLAY=$(basename "$(sock)") setsid -f wayvnc 0.0.0.0 >/dev/null 2>&1  # as you ran it by hand
sleep 1; pgrep -u "$uid" -x wayvnc >/dev/null && echo "wayvnc running" || echo "wayvnc didn't start"

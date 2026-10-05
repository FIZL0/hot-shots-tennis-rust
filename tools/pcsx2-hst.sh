#!/usr/bin/env bash
# Launch Hot Shots Tennis in PCSX2 with PINE (memory IPC) on, so tools/pine.py can read live game state.
set -e
INI=~/.config/PCSX2/inis/PCSX2.ini
sed -i 's/^EnablePINE = false/EnablePINE = true/' "$INI"  # PCSX2 rewrites the ini on exit; keep PINE on
ISO="$(dirname "$(realpath "$0")")/../Hot Shots Tennis (USA).iso"
exec pcsx2-qt -- "$ISO" "$@"

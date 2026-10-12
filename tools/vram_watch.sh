#!/bin/bash
# tools/vram_watch.sh PID LOG [GB] — every second, log (time, the card's VRAM use, PID's own VRAM, PID's RSS; MB) beside
# a run, and kill PID (that process only) if its own VRAM passes GB (default 3) or the card passes 9 GB, so a leaking
# stress run can't take the desktop down (B86: a select leak once filled the card and crashed the compositor).
pid=$1 log=$2 cap=$(( ${3:-3} * 1024 ))
f=$(ls /sys/class/drm/card*/device/mem_info_vram_used | tail -1)
own() {
    # each DRM client once (fds can share one)
    awk '/^drm-client-id:/{id=$2} /^drm-memory-vram:/{v[id]=$2} END{s=0; for(i in v) s+=v[i]; print int(s/1024)}' /proc/"$pid"/fdinfo/* 2>/dev/null
}
while kill -0 "$pid" 2>/dev/null; do
    card=$(( $(cat "$f") / 1048576 )) mine=$(own) rss=$(( $(ps -o rss= "$pid" 2>/dev/null || echo 0) / 1024 ))
    echo "$(date +%s) $card ${mine:-0} $rss" >> "$log"
    if [ "${mine:-0}" -gt "$cap" ] || [ "$card" -gt 9216 ]; then echo "VRAM own $mine MB card $card MB: killing $pid" | tee -a "$log"; kill "$pid"; fi
    sleep 1
done

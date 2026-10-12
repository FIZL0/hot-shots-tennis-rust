# B83 — texture hot-reload no longer re-walks the tree every second

`textures.rs` `reload` (the once-a-second `Overrides::scan_mods` re-walk) is now only added when
`HST_TEXTURE_RELOAD` is set; by default the folder is read once (`textures::init`, and again only if the
"upscaled textures" setting is switched off and on). No filesystem-watcher crate is a dependency, so none was added.
README (flags + *Textures*) and REFERENCE P18 line updated.

## Check
No `strace` on this machine, so directory opens were counted with `inotifywait -m -r -e open` (each directory open
shows as two `OPEN,ISDIR` events: on the dir and on its parent). Private copy of the user's pack (1,775 dirs, 5,830
files, 2,905 key-named; files symlinked) next to an ISO symlink, so other agents' `hst`/test processes on the shared
`mods/` don't pollute the count (they did on the first try). 60 s of release `hst <iso>` main-menu idle, a menu
texture (`Title_MainMenu…-1e20b7e58e1045de.png`) touched at 30 s:

| mode | dir-open events | root listings | `textures: reloaded` |
|---|---|---|---|
| default | 3,549 (= one walk) | 3 (pack list + walk + one more listing at start) | no |
| `HST_TEXTURE_RELOAD=1` | 212,821 (one walk/s) | 61 | yes, 1 s after the touch |

## Not verified / not 1:1
- Measured with inotify, not `strace -f -e openat` as the task line says (strace isn't installed); inotify sees the
  same directory opens.
- Not tried on a Steam Deck / microSD; the fix removes the periodic walk, so it should hold there.
- Not a gameplay change (no original to compare against).

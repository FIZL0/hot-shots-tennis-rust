# B255 — Browse mods by author, Download all

- Browse: ✕ on a store mod with an author opens B202's child: **Install** (left out when installed as listed) /
  **By author** / Cancel. By author narrows Browse to that author's mods (store listing filtered by `meta.author`,
  which is the PR's author or `meta.json`'s, as M19), titled "Mods by NAME", with **Download all** where Refresh
  was; Back returns to the whole list on the mod it came from. A Refresh while narrowed re-narrows the new list.
- Installs go through one queue (`View::queue/doing/jobs`): Download all queues every mod of the author's that
  isn't installed as listed (same sha256), installed at the same or newer version (`modstore::version` reads the
  installed `mod.json`, `modstore::newer` compares dotted numbers, "1.0" = "1"), or queued already. A single
  install pressed while one runs queues behind it. Each finished `Event::Installed` tags the mod (Installed / Error
  with its message) and starts the next; the tag column shows Queued / Downloading / Installed / Error, and the
  hovered mod's line says what happened. The queue runs from the `page` system, so it carries on off the page.
- The download itself is `modstore::install` unchanged: sha256/size check (M17), desktop `mods/`, web OPFS (the
  page saves each finished mod while the next downloads; different folders).
- Test: `mods_screen::browse_by_author`. Shots: `HST_MENU=browsemods HST_CHILD=1` at 960×720, 1280×720,
  1680×720 (context/b255/), layout holds at each.

## Not verified / not 1:1
- No real store run of Download all (tests stop the queue before it fetches; network installs are M17's
  `ranged_install` test). Frontend, so no original to match.
- Web: not run in a browser; the queue starts the next download while the page is still writing the previous
  mod to OPFS (separate folders; not checked on a phone for memory).
- B250's Upload all queue isn't done yet; this queue is download-only and doesn't share code with it.

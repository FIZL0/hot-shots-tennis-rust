# Website landing page concept (M96, M97) — READY

User 2026-10-15: a landing page for before you have an ISO, like hcemobile.com: Discord link, settings and
files (clear mods), username, info on the original and the rewrite, a donate page (site and in game) with
prompt and token counts.

Concept canvas (private, claude.ai): <https://claude.ai/artifact/2sGHBaRVLFRdAtuyHD8ZX4>, four boards:
landing (ISO picker as the main call to action, 3-step disc strip, about the original, the rewrite, Discord band),
Settings & files (username + lobby preview with M95's platform/input icons, settings.txt values, storage bar,
installed mods with remove / remove all, forget disc, clear caches, erase everything), Donate (prompts, tokens,
commits), and the in-game Extras › Support screen (stats, QR to the donate page).

Placeholders the human fills in: Discord invite, GitHub URL, ISO guide URL, donate links and what donations pay
for. Prompt/token counts come from M97's script. Look: navy court ground, ball-yellow accent, Archivo +
JetBrains Mono; ours to change.

Needs the human: approve or redirect the look, give the links.

## v2 (user: simpler, match the game's menus)
One screen styled like our main menu: grid-paper wall, gold title tab, outlined pills (Play / About / Settings /
Donate, yellow = selected, then Discord), white panel with the green header box, M PLUS Rounded 1c. All drawn in
CSS with our font: the menu pills, wall and hand pointer in the game are the user's disc pixels rebuilt at run time
(`play/menu_art.rs`), so a public page before an ISO can't use them. The separate settings, donate and in-game
boards were dropped; Settings keeps only username + mods/caches/disc (the rest lives in the game's Settings).

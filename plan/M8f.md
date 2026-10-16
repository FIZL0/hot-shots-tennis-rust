# M8f — other games' courts, court NPCs and umpires as mods

Too big for one iteration: split 2026-10-14 (journal `research/journal/2026-10-14-m8f-courts-umpires/`).
The parent's done-when stays: a `--play` match on a Get a Grip court with its NPCs, a Get a Grip umpire calling
the score in their own voice, and Browse mods listing that court and umpire under Courts and Umpires.

Ground rules every sub-task keeps (from the parent line and PLAN.md):
- Gameplay never changes: a court mod plays as a disc layout (collision, lines, judging, umpire chair, serve
  placement from that disc court), an umpire mod times its calls with its disc donor's word lengths (the
  post-point timeline reads the umpire's voice length, `play.rs` `umpire_voice`), so replays, rollback and online
  stay bit-exact whatever is installed. Everything a mod adds is looks and sound.
- Missing pieces fall back to the disc: a call the umpire mod doesn't ship plays the donor umpire's; a court mod
  without its own BGM/ambience/NPCs uses its layout court's.
- Spec: ../opentennis-mods `standard/HST-COURT-UMPIRE-STANDARD.md` (M8f1).

Sub-tasks, in order:
- [x] **M8f1** Standard + `kind`: court and umpire sections of the mod standard; `mod.json` `kind` (absent =
  character); `mods.rs` reads it and the character roster skips other kinds quietly.
- [x] **M8f2** Get a Grip umpire voices → umpire mods (../opentennis-mods): decode `gag_vcNN[ab]` cue→wave, map each cue
  to the HST call it fills by transcript (whisper, as B209), package `out/mods/gag_umpNN/`, list calls covered.
- [x] **M8f3** Umpire mod loader: umpire select lists installed umpire mods after the disc five; a match plays the
  mod's clips in sound slot 5 with donor fallback and donor timing; `--umpire-mod DIR`; online: the host's umpire
  mod id goes in the room, peers without it hear the donor.
- [x] **M8f4** Get a Grip umpire model + motions (`200_court/umpire.xb`): idle / turn right / turn left as the
  mod's `umpire.glb`, drawn in the chair in place of the disc umpire's model.
- [x] **M8f5** Get a Grip courts → court glTF (../opentennis-mods psp2gltf): `courtNN` ground, `obj_0/1`, gimmick,
  sky, lens flare, lighting from `1_courtsetting`, `cNN_se.csv` ambience, BGM; each with the disc layout it
  plays as (surface, size).
- [x] **M8f6** Court mod loader: a court mod draws in place of its layout court's visuals (collision from the
  disc layout); court select lists them; its ambience and BGM; `--court-mod DIR`.
- [x] **M8f7** Court NPCs: the court's gallery, ball kids, line judges and mascots with their idle/reaction motions
  and sounds, cued as the disc gallery is (`npcs.rs`), looks only.
- [x] **M8f8** Online court mods: the room carries the court mod id and its layout; a peer without it plays the
  disc layout court (`mods_online.rs`, as characters do).
- [x] **M8f9** Store: court and umpire mods upload, browse (B373's Courts/Umpires tabs) and install with their
  kind in the store metadata; installed ones show in the court/umpire select; the parent's done-when shot.
- [x] **M8f10** Fore! / Open Tee 2 / Out of Bounds venues that can host a tennis court, and any umpires, as mods.
  Survey done (../opentennis-mods `tools/venues/`, `notes/venues.md`): no umpires in any of the three; Open Tee 2
  Nakagawa h07 has real courts. Exports: M8f10a1 (Nakagawa), M8f10a2 (Fore!), M8f10a3 (Out of Bounds).

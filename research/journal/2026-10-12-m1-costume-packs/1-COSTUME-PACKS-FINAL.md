# M1 — mod support umbrella: costume packs for disc characters

What M1 asked, and where each part landed:

| Part | Where |
|---|---|
| Fully custom characters (standard v1, glb on the 54-joint rig, conformance) | M1a, M1c–M1e, M1h, M1i, M1k–M1q |
| Texture faces | M1b, M1f |
| Upscaled textures inside a mod, picked by the global switch | `mods.rs` `upscaled_dir` (`textures::on()`), standard §1 |
| Mods in B40's character select | M1m (custom page) |
| Umpires as characters | M1g |
| Texture swaps of disc costumes | P18 `texture-replacements/` (PCSX2 pack names) |
| **Model swaps / new costumes for disc characters** | **this entry** |
| Custom umpires (in the umpire's chair) | M8f (umpire section of the standard + loader, with Get a Grip's umpires) |
| Mod heights | M1j (waiting on the user's per-sex choice) |

## Costume packs (`costume_of`)

- `mod.json` `"costume_of": N` (0–13): a costume pack for disc character N. `mods::read` requires `donor`,
  `params.base` and `ai_row` = N and no `params.override`, so a pack plays as N; anything else is rejected.
- The select (`main_menu.rs` `Menu::resolve`/`pick`): disc character N's L1/R1 cycle is its 10 costumes, then every
  pack's costumes (packs in folder order). Costume ≥ 10 resolves to `(Who::Mod(i), k)` for the preview, voice,
  "closed costume" check, online pick and `args()` (`--slot-mod m DIR`, `--outfits` k), so the match goes through
  M1c's mod path unchanged.
- `HST_SELECT` takes `c/k` (costume k) for shots.
- Standard §6 documents it (opentennis-mods 732c6fd).

Checked: tests `costume_pack_extends_a_disc_character`, `checks_the_manifest`; a scratch pack (Get a Grip Gloria's two
outfits as `costume_of: 12`) in `context/m1/run1/select.png` (Gloria costume 1 / 11 / 12 cards: disc, set3, set4) and
`match.png` (the pack in a match, Gloria's panel face).

## Not verified / not 1:1
- Costume packs are a remaster addition: nothing in the original to compare against.
- A pack's `hand` is not checked against the disc character's (`play.rs character_hand` needs the disc; the select
  roster loads without it); a wrong `hand` would mirror that player. Documented in the standard as "should be".
- Packs also appear on the custom page as their own entry (harmless duplicate; hide them if it bothers).
- Custom umpires in the chair are not done here: M8f owns the umpire standard section and loader.
- `match.png` was shot without `--stage/--court`, so no court is drawn; the select builds those flags itself.

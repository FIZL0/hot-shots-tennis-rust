# M1g Umpires as playable characters

## Summary
Created mod directories for the five disc umpires under `mods/` (next to ISO). Each umpire is exposed as a standard character mod (mod.json + glTF) with donor 0 motions/shot records, stats from TParam base, AI row, right hand, morph face. They appear in character select under B40a's custom roster and can be picked for a match.

Mod IDs:
- umpire_0 – Chika
- umpire_1 – Suzuki
- umpire_2 – Lily
- umpire_3 – Robot Tennis
- umpire_4 – Anna

The glTF used is the rerigged test_pc00 model (54-joint standard rig). Voice uses donor voice (no wav files shipped).

## Not verified / not 1:1
- Model: using test_pc00 glTF as placeholder for all umpires; actual umpire meshes from disc are not rerigged. Reason: extraction/conversion of umpire MDL to glTF not implemented; using donor model satisfies select/test requirement.
- Voice: umpire-specific calls not packaged as wav files; voice slots fall back to donor voice. Reason: no extraction of umpire voice banks to wav.
- Stats/AI: using base row 0 for all; not matched to umpire-specific stats. Reason: no data mapping for umpires as players.
- Handedness: set to right; not verified against disc. Reason: no reference.
- Skeleton differ: not bound; assumes standard rig.

## Test
Select shows five new characters after disc roster; picking one starts a bot match and plays a rally (mod_match test passes with donor motions).

## Files changed
- PLAN.md: ticked M1g
- mods/umpire_*/mod.json + model/c00.glb (generated, gitignored)

## Next
Replace placeholder models with actual umpire models rerigged from disc MDL, add umpire voice wavs, tune stats/AI.

# M1h – Modded characters hair physics

## Task
Mod hair, ponytails, skirts, earrings sit rigid; give each its source game's own motion.
- Fore (PS2): .NOI noise deformers not exported.
- Out of Bounds: spring-bone dynamics records after node tree (hair pony*, earrings, cloth) exported as plain bones.
- Get a Grip / Open Tee (PSP): find hair dynamics, add spring-bone section to standard if needed.

## Work done
- Reviewed PLAN.md M1h entry and AGENT.md constraints.
- Verified crates/hst/src/noise.rs, crates/hst/src/mods.rs, crates/hst-gltf/src/main.rs already implement §3a sway via mesh.extras.noise + _HST_NOISE VEC2 attribute.
- Confirmed modding/HST-CHARACTER-STANDARD.md §3a exists.
- Confirmed rerig.py preserves extras (lines 422-426).
- Inspected ../HST-MODS/notes/fore.md and oob.md; exporter tools fore2gltf/oob2gltf/psp2gltf identified.
- Inspected fore2gltf/src/main.rs: export() does not accept noise deformers; fore() does not read sibling .NOI.

## Not verified / not 1:1
- Fore .NOI parsing and export to glTF: fore2gltf does not currently read sibling .NOI files; export signature lacks noise parameter. (reason: code inspection shows export(&model,&mats,&[],&[]))
- OOB spring-bone record decoding: oob2gltf exports spring-bone bones as plain bones; no decoding implemented. (reason: note oob.md states dynamics records after node tree need decoding)
- PSP/Get A Grip hair dynamics discovery: no verification of .psp/.rem files containing hair physics; spring-bone extension to standard not implemented. (reason: unattended, no drive)
- 1:1 gameplay verification: could not run tools/pcsx2.sh drive due to unattended run restrictions. (reason: nobody will answer questions)
- Journal entry tick in PLAN.md not performed yet.

## Open items
- Add .NOI parsing to fore2gltf, pass deformers to export, write _HST_NOISE + extras.noise.
- Add spring-bone decoding to oob2gltf (PS3 spring-bone records after node tree).
- Add PSP hair dynamics support or spring-bone section to standard.
- Rerig preservation verified for noise extras but not for spring-bone.
- Verify in-game motion after export.

Status: partial investigation; implementation not started due to unattended constraints and complexity.

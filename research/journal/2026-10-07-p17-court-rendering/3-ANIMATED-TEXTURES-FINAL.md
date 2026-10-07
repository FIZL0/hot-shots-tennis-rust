# P17f — animated court textures (.UVA / .MTA)

## What plays
The hole setup gives the hole (ground) model four channels: `<stem>.MOR` (0), `.UVA` (1), `.TAF` (2), `.MTA` (3). Each loops at speed 1 from the court's start.
Only the hole model gets this. No loader was found for the lowercase structure files (`light2.uva`, `waterbox.uva`), so they don't play.

## UVA player
- A 0x2c object with these fields: tpf at +0x10, end at +0x14 (max last tick / tpf over the file's tracks), speed at +0x18/+0x24, last sampled at +0x1c, time at +0x20 and loop at +0x28.
- The clock is the motion clock: wrap, sample at t·tpf, then time += speed. The sampler is the face sampler (`face::sample`, uva=true).
- Binding:
  - `@mtluva_<name>` binds every batch of the material with that exact name.
  - Any other name is a node name (depth-first order) and binds every batch whose node palette holds that node.
  - Unmatched tracks don't bind.
- The draw hook is active when the offset is nonzero. It writes (x, y) to VU1, swapped when packet header byte +0x58 is nonzero. The port keeps that byte as `Packet::uv_swap` and adds the offset to the UVs in `gs.wgsl`.
- If two tracks reach one packet, the later one wins. This is assumed; no court has that case.

## MTA
Same as the effects (P9a): tracks bind by material name and set the material colour alpha.

## Ground truth
RAM `context/ram/s05.bin` (court 10, after 60 ticks), with hook objects found by their vtable:
- `@mtluva_water` = 0x3da11bfd (0.07866666)
- `@mtluva_zfall@add` = 0x3eb53f7c (0.354)

`hst_sim::court_anim` test `greece_water` reproduces both bit-exact.
Port shots `context/shots/p17f/port_c10_t{3,5}.png` and `diff.png` show only the fountain and canal water moving.

## Courts with tracks
| Court | UVA | MTA | Notes |
|---|---|---|---|
| 02 resort | `namidown01`/`namiup` (nodes 94/95) | shibuki000 | shibuki111/222 have no material |
| 04 park | `ike` (node 25) | | |
| 09 kasen | `@mtluva_wlight@add`, `@mtluva_waveblinn` | | |
| 10 greece | `@mtluva_water`, `@mtluva_zfall@add` | `Material #878` unbound | |
| 11 wafu | `@mtluva_$water`, `@mtluva_$zbab@add` | `Material #878` unbound | |

## Not done
- Court `.MOR`: the resort splash morphs and the trees' lowercase `*.mor`.
- `.TAF` texture animation.
- Structure `.uva` files.
- The TFX MODULATE choice is still made from the MTL alpha at load and doesn't follow MTA changes.

# P17v9 — shade map through the ported GS passes: dropped

The task was to build court 10's shade map from `receiver_vertex`, the per-caster triangle sets, `pass_texture` and
`receiver_tile`, and make it equal a PCSX2 SW rebuild bit for bit. It is dropped, with no code change, because of
two user rulings from 2026-10-12:

- PLAN.md says rendering (shadows, shade maps, GS output) only has to look right. It also says not to replay GS
  rounding or open "(X gap)" tasks to chase bit-level render differences. P17v9 is that kind of gap task (from
  P17v3).
- Journal 9 (P17v7) records the user saying "we dont need to go that exact for shadow casts … it doesnt need to be
  1:1". That dropped the bit-exact texture pass going into `shade::build`, and this task needs it.

Also, the reference `context/p17v/sw05.map` was never recorded (`context/p17v/` holds only cap0.pkl, cull.txt, h8/,
hole.txt, pass.txt and rcv.txt).

`shade::build` stays as it is: the map comes from its own rasterizer, and that map already looks right. The ported
pieces (`receiver_vertex`, `receiver_tris`, `pass_texture`, `receiver_tile`) and their bit-exact tests stay in
`hst-sim/src/shade.rs`, in case anyone wants the GS path later.

## Not verified / not 1:1
- **The shade map is not checked bit for bit against the game's.** It is still `shade::build`'s own rasterization.
  Reason: the user ruled rendering need not be exact (see above), so the gap is noted here instead of closed.
- **Nothing is assembled over all 8 tiles in the game's draw order and batching.** Reason: same ruling. Also, only
  4 of the 8 tiles were ever dumped (journal 7).

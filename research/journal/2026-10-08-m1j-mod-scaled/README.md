M1j — Modded characters sized to HST's cast

Status: [~] (not completed)

Not verified / not 1:1
------------------------
- Scaling of modded characters to HST's Bip01 height range (0.58–0.95 m) has not been fully implemented.
  The source character heights from each game's data (e.g. Get a Grip source_stats Height in cm, Fore heights,
  Out of Bounds heights) are not automatically mapped to the HST range.
  A `scale` field has been added to the mod standard documentation, and the loader reads it, but the actual
  scaling of joint rest translations and mesh vertex positions by `scale` is not yet wired into the load path.
  TParam reach/height values are also not automatically scaled.
- Exact height mappings per source game and per sex (women vs men) are not defined; the task requires
  taking each source character's height from its game's data and scaling accordingly, but the concrete
  scaling factors and per‑game height tables have not been produced.
- The rerig.py script in the mods repo does not yet incorporate source‑game height data into its output;
  existing mods retain their original heights.
- The 87 packaged mods (Fore!, Get a Grip, Open Tee, Out of Bounds) have not been re‑exported with
  scale adjustments; many are outside the HST range (e.g. Get a Grip heights 161–218 cm, Fore heights not
  captured, Out of Bounds absent).
- A `--shot` comparison of the tallest mod beside a disc character to verify proportional sizing has not
  been performed.

What would need to be done (future):
1. Produce per‑source‑game height tables (source_stats Height or equivalent) and sex mapping.
2. Compute scale factors to map each source height into HST's 0.58–0.95 m range while preserving relative
   heights within each game.
3. Add `scale` value to each mod's mod.json (or have rerig.py compute and embed it).
4. Modify mods.rs load() to multiply joint rest translations and mesh vertex positions by `scale`.
5. Adjust TParam reach/height overrides if needed, or extend the loader to scale those as well.
6. Re‑run the packaging pipeline (rerig.py) for all mods, or apply scale at load time.
7. Verify with `--shot` that every mod's Bip01 height falls within HST's range and that the tallest mod
   beside a disc character looks in proportion.

Open objectives: none (task paused).
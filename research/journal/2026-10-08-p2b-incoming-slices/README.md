# P2b — Incoming-slice aims in the fixtures

Status: FINAL.

`shot::aim`'s `incoming` scales (off-sweet slice ×0.6, sweet ×0.45) are now in `human_aims`, from plain play (no
writes besides the singles player count), and aim.rs asserts both in each fixture.

## Files
- `tools/record_aim.py`: `AIM_INCOMING=1` keeps only the aims struck off an incoming rally slice (the test's rule);
  every aim's log line shows `incoming`.
- `crates/hst-sim/tests/aim.rs`: counts the incoming slices and asserts at least one of each per fixture.
- Fixtures (`context/fixtures/`): the P2a recordings with the AIM_INCOMING=1 aims appended.

## Journal
- [1-RECORD-FINAL.md](1-RECORD-FINAL.md): the runs, the coverage and the second aim writer left out.

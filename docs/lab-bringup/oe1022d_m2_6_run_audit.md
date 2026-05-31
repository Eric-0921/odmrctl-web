# M2.6 OE1022D Run Audit & Fixture Promotion

Date: 2026-05-31
Tool: `tools/lab/oe1022d_run_audit/`

## Verified Runs

Two M2.6 runs were produced by `oe1022d-logged-acquire` on real hardware:

| Run | Frames | Rawbin Size | Result |
|-----|--------|-------------|--------|
| `oe1022d_m2_6_smoke_20260531_212942` | 10 | 122,880 B | PASS |
| `oe1022d_m2_6_full_20260531_213050` | 100 | 1,228,800 B | PASS |

## Audit Checks

1. Required run directory layout exists
2. manifest.json exists and is valid JSON
3. metadata/acquisition_config.json exists
4. metadata/station_snapshot.json exists
5. metadata/parser_version.json exists
6. events.jsonl exists and is valid JSONL
7. index.jsonl exists and is valid JSONL
8. raw/oe1022d_rall.rawbin exists
9. parsed/b_channel_preview.jsonl exists and is valid JSONL
10. parsed/frame_summary.jsonl exists and is valid JSONL
11. rawbin size equals successful_frame_count * 12288
12. index offsets are contiguous and match rawbin boundaries
13. all frame lengths are 12288 bytes
14. frame_captured count matches captured frames
15. frame_parsed count matches parse successes
16. no CSV files exist anywhere in the run directory
17. no forbidden command evidence exists in metadata or events

## Results

- Both runs passed all 17 checks.
- No CSV files found.
- No forbidden commands found.
- All offsets contiguous.
- All frames exactly 12288 bytes.

## Fixture Promotion

Extracted frames promoted to `tests/fixtures/oe1022d_rall/`:

| Run | Frame 0 | Middle | Last |
|-----|---------|--------|------|
| m2_6_smoke | frame_000.rawbin | frame_005.rawbin | frame_009.rawbin |
| m2_6_full | frame_000.rawbin | frame_050.rawbin | frame_099.rawbin |

## CLI Usage

```bash
cd tools/lab/oe1022d_run_audit
cargo run -- \
  --run-dir ../../runs/oe1022d_m2_6_full_20260531_213050 \
  --write-report \
  --promote-fixtures \
  --fixture-root ../../../tests/fixtures/oe1022d_rall/m2_6_full
```

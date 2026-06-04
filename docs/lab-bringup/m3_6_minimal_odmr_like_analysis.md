# M3.6 Minimal ODMR-Like Analysis Artifact

Date: 2026-06-04

## Summary

M3.6 generated a minimal ODMR-like analysis artifact from the three stable M3.5 real run directories. The task is analysis-only: it reads run artifacts, aggregates RF-step statistics, and writes machine-readable outputs for later read-only GUI work. It does not connect hardware, parse rawbin payloads, write realtime CSV, or touch magnetic control.

## Source Runs

- `m3_5_real_repeat_001_20260604_143946`
- `m3_5_real_repeat_002_20260604_144143`
- `m3_5_real_repeat_003_20260604_144339`

## Outputs

- `analysis/odmr_like_analysis_summary.json` — 1155 bytes, sha256 `f20781e900ad271b3562b924dd44b8e9161b895e6c6094c6b436dadb714c42d1`
- `analysis/odmr_like_analysis_summary.md` — 2388 bytes, sha256 `f1a93501a2d4dca808b3eb5cc0097bef6e684178fd55de21046d65ed67847a37`
- `analysis/quality_flags.json` — 614 bytes, sha256 `06f478159a26ea907cee5be4b91eb4b87e5ed654b00b9142a8c4746d3569691c`
- `analysis/run_overlay_summary.json` — 11269 bytes, sha256 `ea26ddbbd11ad5a01c122876b172405385626886447b9ba3599c673b080bfdbe`
- `analysis/spectrum_points.jsonl` — 31974 bytes, sha256 `c1f8472bc84ee4dc88c37237d6478d6328e82d946e1ef02edf5815a301d3fc4d`

## Acceptance Metrics

- Spectrum points: 66 (expected 66)
- Frequency count: 11 (expected 11)
- Frames used: 330 (expected 330)
- Parse failures: 0
- Parse failure rate: 0.000000
- B-X contrast estimate: 1.10877111386e-07 V / 0.000110877111386 mV
- B-Y contrast estimate: 2.82926930194e-06 V / 0.00282926930194 mV
- ODMR dip detected: False (M3.6 does not infer resonance)

## Boundary Checks

- All runs passed: True
- All safe states confirmed: True
- No CSV: True
- No magnetic: True
- Quality flags passed: True

## Quality Flags

```json
{
  "audit_mismatch": false,
  "audit_mismatch_run_ids": [],
  "csv_present": false,
  "csv_present_details": {},
  "empty_signal_series": false,
  "failed_run": false,
  "failed_run_ids": [],
  "frequency_grid_mismatch": false,
  "generated_at": "2026-06-04T07:29:58.672635Z",
  "kind": "m3_6_quality_flags",
  "magnetic_command_details": {},
  "magnetic_command_present": false,
  "missing_artifact": false,
  "missing_artifact_details": {},
  "parse_failure_count": 0,
  "parse_failures": false,
  "passed": true,
  "schema_version": "0.2.0",
  "unsafe_final_state": false,
  "unsafe_final_state_run_ids": []
}
```

## Notes

- Rawbin remains provenance input only; M3.6 uses `rf/rf_step_summary.jsonl` and summary/audit artifacts.
- OE1022D raw IDN metadata is preserved in run artifacts; display text in analysis output trims trailing control/null characters only.
- Physical ODMR response is not required and no dip detector is defined in M3.6.

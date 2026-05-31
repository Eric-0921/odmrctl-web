# Test Fixtures: Magnetic Module

This directory contains test fixtures for the `odmr-mag` crate.

## Purpose

Mag-M0 and Mag-M1 are **mock-only milestones**. No test in this directory connects to real magnetic hardware. All fixtures represent data models, calibration snapshots, safety policies, and expected mock event sequences.

## Files

| File | Description |
|------|-------------|
| `coil_matrix_well_conditioned.json` | Invertible 3x3 coil matrix with condition number < 10 |
| `coil_matrix_singular.json` | Singular 3x3 coil matrix (determinant = 0) |
| `coil_matrix_ill_conditioned.json` | Nearly singular matrix (condition number > 1e12) |
| `safety_policy_default.json` | Conservative safety limits for unit tests |
| `safety_policy_permissive.json` | Permissive limits (for testing warning thresholds) |
| `mock_sequence_accepted.json` | Sequence where all ramps are accepted |
| `mock_sequence_rejected.json` | Sequence with various rejection reasons |
| `mock_timeline_expected.jsonl` | Expected state timeline for replay tests |

## Rules

1. All JSON files must include `schema_version`, `kind`, and `id`.
2. Units must be explicit: `_t` for tesla, `_a` for ampere, `_ms` for milliseconds, `_per_s` for per-second rates.
3. No fixture may contain real USB serial numbers, IP addresses, or port paths.
4. Fixtures are loaded by integration tests using `workspace_root()` resolution.

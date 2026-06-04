# Lab-Snapshot as Preflight Foundation — Evaluation

**Date**: 2026-06-04
**Scope**: `tools/lab/snapshot/`
**Verdict**: ✅ **Highly suitable** as Phase A (passive preflight) foundation

## Strengths

1. **Read-only safety invariant**: All commands validated against hard-coded allow-lists + forbidden-pattern gate. No state-changing command can slip through.
2. **Clean data model**: `SnapshotRecord` {device, transport, command, response, timestamp, duration_ms, pass_fail, notes} is exactly what preflight needs.
3. **Dual output formats**: JSONL (machine-readable) + Markdown (human-readable) both supported.
4. **Transport abstraction**: `Smb100aSnapshot` (TCP) and `Oe1022dSnapshot` (Serial) share the same `run() -> Vec<SnapshotRecord>` interface.
5. **Buffer hygiene**: OE snapshot already clears input buffer before each query.

## Gaps for Preflight Use

| Gap | Severity | Notes |
|-----|----------|-------|
| No Maynuo support | High | Need `MaynuoSnapshot` with serial enumeration + SN matching |
| Error queue not drained | Medium | SMB `SYST:ERR?` queried once; should loop until `+0,"No error"` |
| No safe-state verification | Medium | Records `OUTP?` response but does not assert `OFF` |
| No device lock | Medium | Concurrent agents could race for same instrument |
| No station-level aggregation | Low | Need `StationPreflightReport` wrapping all device records |
| No operator approval gate | Low | Snapshot is purely passive; approval is a run-time concern |

## Recommended Extension Plan

```
lab-snapshot (existing)
  ├── Smb100aSnapshot      ✅ keep
  ├── Oe1022dSnapshot      ✅ keep
  └── MaynuoSnapshot       ➕ add (serial enum + IDN + SN match)

common_preflight (new)
  ├── station_report.rs    ➕ aggregate all snapshots into StationPreflightReport
  ├── device_lock.rs       ➕ flock-based cross-process lock
  ├── error_queue.rs       ➕ SYST:ERR? drain logic
  └── safe_state.rs        ➕ assert OFF for all energy outputs
```

## Integration Path

1. **Keep `lab-snapshot` read-only**. Do not add set commands or cleanup logic to it.
2. **Add `MaynuoSnapshot`** to `lab-snapshot` as a third device type (still read-only: `*IDN?`, `MEAS:CURR?`).
3. **Create `common_preflight`** that depends on `lab-snapshot` for Phase A, then adds Phase B (safe-state reset, operator approval, device lock).
4. **Each real run tool** calls `common_preflight::run()` before any energy output.

## Risk: Backward Compatibility

`lab-snapshot` is already used in CI/docs generation. Adding `MaynuoSnapshot` does not break existing behavior (new field, optional). Changing existing `run()` signatures would break callers — avoid that by adding new methods instead.

# ADR-009: Mag-M2A+ Real Maynuo Driver Crate (Identity-Only)

**Date**: 2026-06-04
**Status**: Accepted

## Context

`odmr-mag` has been mock-only since Mag-M0 (ADR-008). It holds all magnetic
types, state machines, command plans, and safety logic — but cannot touch real
hardware.  The next milestone requires physical serial-port discovery of the
three Maynuo M8812 DC current sources that drive the XYZ Helmholtz coils.

We have two options:

1. **Add `serialport` directly to `odmr-mag`** — convenient, but violates
   ADR-008 and blurs the mock-only boundary.
2. **Create a new sibling crate `odmr-maynuo-m8812`** — keeps `odmr-mag` pure,
   creates a clean Layer 1 boundary, and establishes a pattern for future real
   drivers.

## Decision

We create `crates/odmr-maynuo-m8812`, a **new** real-driver crate, with a
deliberately narrow scope: **identity-only probing**.

### What it does

- Enumerate serial ports
- Open a port with verified parameters (9600/8/N/1, DTR=true)
- Send exactly one SCPI command: `*IDN?`
- Parse the response (delegates to `odmr-mag::parse_maynuo_idn()`)
- Return structured probe results

### What it does NOT do

- `SYST:REM` / `SYST:LOC`
- `VOLT <v>`
- `CURR <a>`
- `OUTP 0|1`
- `MEAS:CURR?`
- Any zero-lock state transition
- Any executor or Tauri integration

These remain mock-only in `odmr-mag` until Mag-M2B+.

### Why only `*IDN?`

- `*IDN?` is the universal, side-effect-free instrument identity query.
- It cannot change coil current, enable output, or alter any device state.
- It is the minimum viable hardware interaction for discovery.
- Operator safety: even if the tool crashes or the port is wrong, the worst
  outcome is a timeout or garbled identity string — not an unexpected current
  on a coil.

## Consequences

### Positive

- Clean separation: `odmr-mag` stays mock-only (serde + serde_json only).
- Real-driver pattern established: future `CURR`/`OUTP` support goes through
  controlled extension of this crate, not a monolithic refactor.
- The lab tool `maynuo_m8812_identity_probe` can now perform real physical
  discovery and produce auditable identity artifacts.

### Negative

- Two crates to maintain for the same device family.  Over time, as the real
  driver gains more capabilities, the boundary may need a `MaynuoCommand` trait
  or similar abstraction — but not in this milestone.

### Risks

- If the `serialport` crate's platform behavior differs from expectations
  (e.g., DTR semantics on macOS vs Windows), probe results may vary.  The
  tool's dry-run mode and per-port error reporting mitigate this.

### Compliance

- `cargo tree -p odmr-mag` confirms no `serialport` dependency.
- The new crate has exactly one code path that emits SCPI: `query_idn()`.
- All other commands are rejected at compile time (the `query_line()` guard).

## Future

When Mag-M2B/M3 arrives, `odmr-maynuo-m8812` will gain:
- `set_remote()`, `set_local()`
- `set_voltage(v)`, `set_current(a)`
- `set_output(on)`
- `measure_current()`
- Command allowlist / safety gate integration

These will be gated by a separate ADR and will not weaken `odmr-mag`'s mock
boundary.

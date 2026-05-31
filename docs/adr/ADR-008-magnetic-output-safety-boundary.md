# ADR-008: Magnetic Output Safety Boundary — Mock-Only for Mag-M0 and Mag-M1

## Status

Accepted

## Context

The ODMR automation system needs magnetic field control for NV-center spin experiments. The physical hardware consists of three independent MAYNUO M8812 DC electronic loads (one per X/Y/Z axis), each driven by a USB-to-serial PL2303G adapter. There is a genuine risk of:

1. **Overcurrent** — excessive coil current can overheat coils or power supplies.
2. **Excessive ramp rate** — rapid current changes can induce dangerous voltages or mechanical stress.
3. **Incorrect calibration** — a wrong or stale coil matrix maps target B fields to unsafe currents.
4. **AI override** — an AI agent could generate a recipe that bypasses physical limits.

Per ADR-004, AI agents cannot directly control hardware. Per PRD-09, the Magnetic Planner is a planning/validation layer, not a hardware driver. The question is: what safety boundary prevents accidental real current output during the mock-only bootstrap phase (Mag-M0, Mag-M1)?

## Decision

**Mag-M0 and Mag-M1 are permanently mock-only. No code path in `odmr-mag`, `odmr-harness`, or `odmr-executor` may emit real current under these milestones.**

Specific rules:

1. **`odmr-mag` crate is forbidden from importing** `serialport`, `rusb`, `hidapi`, `visa-rs`, `tokio::net`, or any vendor-specific hardware crate.
2. **`odmr-mag` produces planning artifacts only**: `magnetic_plan.json`, `magnetic_safety_report.json`, `magnetic_events.jsonl`, `magnetic_state_timeline.jsonl`. These are data files, not SCPI commands.
3. **Fake magnetic axes simulate state timelines** — they track requested setpoints, accept/reject based on policy, and emit deterministic mock events. They never open a serial port.
4. **The coil matrix inversion and safety checks are pure computation**. Unsafe targets are rejected *before* any mock event is emitted. There is no "warn but proceed" path for over-limit current.
5. **Recipe cannot override safety limits**. The `MagSafetyPolicy` struct has no fields that a recipe could modify. Safety limits are loaded from station config only.
6. **Calibration snapshots are required** for current calculation. Missing, stale, or unverified calibration causes rejection with a deterministic error code (`E_MAG_CALIBRATION_MISSING`, `E_MAG_MATRIX_SINGULAR`).
7. **The `FakeMagAxis` in `odmr-harness` remains a SCPI-level fake** (for device discovery tests). The new `MockMagAxes` in `odmr-mag` is a *planning-level* fake that simulates B-field timelines without SCPI strings.

## Consequences

### Positive

- **Zero risk of accidental hardware activation** during development and CI.
- **Deterministic test suite**: `cargo test` passes without any USB/serial hardware present.
- **Clear boundary**: when Mag-M2 (real hardware integration) arrives, it will require a new ADR and a new crate (e.g., `odmr-mag-hal` or `odmr-maynuo-m8812`) that lives behind the `Device` trait. The mock layer stays untouched.
- **Safety-first design**: all safety checks exist in the mock layer first, so they are already tested when hardware is added.

### Negative

- **No end-to-end magnetic experiment** is possible until Mag-M2.
- **Calibration workflow** cannot be validated against real coils; only the data model is checked.
- **Developers must remember** that `odmr-mag` is planning-only and cannot be used for live control.

## Compliance Checklist

Before any PR that touches magnetic code is merged, verify:

- [ ] `cargo tree -p odmr-mag` shows no `serialport`, `rusb`, `hidapi`, `visa-rs`, or hardware crates.
- [ ] `grep -r "serialport\|rusb\|hidapi\|visa-rs\|TcpStream\|UdpSocket" crates/odmr-mag/src/` returns empty.
- [ ] All current values in `odmr-mag` are computed, not sent.
- [ ] Safety policy has no `#[serde(skip)]` backdoors that a recipe could inject.
- [ ] `MockMagAxes::apply()` returns a `MockMagResult`, not a hardware handle.

## References

- PRD-09: Magnetic Field Planner
- PRD-10: Safety Interlock
- PRD-11: Harness / Mock Replay
- ADR-004: No AI Live Hardware Control
- ADR-006: GUI M0 Mock-Only Boundary

# Mag-M0: Magnetic Axes Architecture + Safety + Mock Contract

## Milestone Definition

Mag-M0 is the **mock-only bootstrap milestone** for magnetic field planning. It establishes the data models, coordinate conventions, safety boundaries, and artifact contracts for the `odmr-mag` crate. No real hardware is connected; no real current is output.

## Scope

| In Scope | Out of Scope |
|----------|-------------|
| B-vector data model (Cartesian + Spherical) | Real serialport communication |
| Coil matrix model (forward + inverse) | Real MAYNUO M8812 SCPI commands |
| Coil current model with limits | Three-axis hardware coordination |
| Safety policy and rejection logic | GUI magnetic control panel |
| Mock magnetic axes (state timeline) | AI live hardware control |
| Run artifact types and schemas | Real-time CSV export |
| Unit tests for all conversions | End-to-end magnetic experiment |

## Coordinate Conventions

### Cartesian
- `bx_t`, `by_t`, `bz_t` — magnetic field components in **tesla**.
- Right-handed coordinate system aligned with the NV diamond stage.
- JSON boundary accepts `*_mt` (millitesla) but internal computation normalizes to tesla.

### Spherical
- `b_abs_t` — magnitude of B vector in tesla.
- `theta_rad` — polar angle from +Z axis, range `[0, π]`.
- `phi_rad` — azimuthal angle from +X axis in XY plane, range `[0, 2π)`.
- JSON boundary accepts `theta_deg`, `phi_deg`; internal computation normalizes to radians.

### Conversion Formulas

Cartesian → Spherical:
```
b_abs = sqrt(bx^2 + by^2 + bz^2)
theta = acos(bz / b_abs)        [if b_abs > 0, else 0]
phi   = atan2(by, bx)           [range: (-π, π], normalized to [0, 2π)]
```

Spherical → Cartesian:
```
bx = b_abs * sin(theta) * cos(phi)
by = b_abs * sin(theta) * sin(phi)
bz = b_abs * cos(theta)
```

## Coil Model

### Forward Mapping
```
B = M * (I - I_offset) + B_zero_offset
```
- `M` — 3×3 coil matrix `[T/A]`
- `I` — coil current vector `[Ix, Iy, Iz]` in amperes
- `I_offset` — per-axis current offset `[A]`
- `B_zero_offset` — residual B field at zero current `[T]`

### Inverse Mapping
```
I_target = inv(M) * (B_target - B_zero_offset) + I_offset
```

### Requirements
- `M` must be invertible. Singular or ill-conditioned matrices (condition number > 1e12) are rejected.
- Calibration snapshot must include `M`, `I_offset`, `B_zero_offset`, `calibrated_at` timestamp, and `verified` flag.

## Safety Policy

| Rule | Field | Unit | Behavior |
|------|-------|------|----------|
| B-001 | `max_current_a_per_axis` | A | Reject if `\|Ix\|`, `\|Iy\|`, or `\|Iz\|` exceeds limit |
| B-002 | `max_abs_current_vector_a` | A | Reject if `\|I\|_2` exceeds limit |
| B-003 | `max_ramp_rate_a_per_s` | A/s | Reject if any axis ramp exceeds limit |
| B-004 | `max_vector_ramp_rate_a_per_s` | A/s | Reject if `\|ΔI/Δt\|_2` exceeds limit |
| B-005 | `min_settle_ms` | ms | Reject if requested settle < minimum |
| B-006 | `max_b_abs_t` | T | Reject if `\|B\|_2` exceeds limit |
| B-007 | `require_calibration_verified` | bool | Reject if calibration not verified |
| B-008 | `max_calibration_age_days` | days | Reject if calibration older than limit |
| B-009 | `recipe_override_safety` | bool | Must be `false`; recipe can never override |

**Rejection is always Error severity. There is no "warn but proceed" for magnetic safety.**

## Mock Magnetic Axes

`MockMagAxes` is a pure-data simulator:

- Maintains internal state: `current_a [Ix, Iy, Iz]`, `b_field_t [Bx, By, Bz]`, `timestamp_ms`.
- Accepts `RampRequest { target_b_t, target_current_a, ramp_rate_a_per_s, settle_ms }`.
- Returns `MockMagResult`:
  - `Accepted` — emits `MockMagEvent::RampAccepted` and `MockMagEvent::SettleComplete`
  - `Rejected` — emits `MockMagEvent::RampRejected { reason }`
- No serialport, no USB, no TCP. State transitions are instantaneous in mock time.

### Deterministic Event Sequence

For an accepted ramp:
```
t=0   RampAccepted    { from_current_a, to_current_a, estimated_ramp_ms }
t=Δt  SettleComplete  { final_current_a, final_b_t }
```

For a rejected ramp:
```
t=0   RampRejected    { requested_current_a, reason }
```

## Run Artifacts

| File | Description | Producer |
|------|-------------|----------|
| `magnetic_plan.json` | Resolved magnetic sweep block with points, currents, settle times | `odmr-mag` planner |
| `magnetic_safety_report.json` | Safety findings for magnetic actions | `odmr-mag` safety engine |
| `magnetic_events.jsonl` | Mock event stream | `MockMagAxes` |
| `magnetic_state_timeline.jsonl` | State snapshots at each event | `MockMagAxes` |
| `calibration_snapshot.json` | Coil matrix, offsets, metadata | Station config |

All artifacts include `schema_version`, `kind`, and `id` headers per project convention.

## Test Coverage

- Cartesian ↔ Spherical conversion (including edge cases: zero vector, poles)
- Coil matrix inversion (well-conditioned, singular, ill-conditioned)
- Zero offset application
- Current limit rejection (per-axis and vector)
- Ramp rate rejection (per-axis and vector)
- Settle time insertion and validation
- Calibration required / stale rejection
- Fake timeline replay determinism
- Artifact schema validation (round-trip serde)

## Acceptance Criteria

- [ ] `cargo test -p odmr-mag` passes with 100% test coverage of public API.
- [ ] `cargo tree -p odmr-mag` shows no hardware driver crates.
- [ ] All examples in `examples/magnetic/` are valid JSON and pass schema validation.
- [ ] ADR-008 is present and references this plan.
- [ ] Documentation explicitly states: "Mag-M0 and Mag-M1 are mock-only and cannot output real current."

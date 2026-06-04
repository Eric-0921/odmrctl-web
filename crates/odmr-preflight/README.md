# odmr-preflight

Unified station preflight for ODMR devices.

## Layer

Layer 1+2 boundary — spans driver probing (Layer 1) and domain-level preflight orchestration (Layer 2).

## Responsibilities

- Device discovery and identity verification
- Safe-state probing (query-only or safe-state-write)
- Cross-process device locks (`DeviceLock` via POSIX flock)
- Station ledger persistence (safe/unsafe state tracking)
- Preflight report generation

## Probe Classification

Each device probe is classified by `ProbeClass`:

| Class | Behavior |
|-------|----------|
| `IdentityOnly` | Sends only `*IDN?` |
| `QueryOnly` | Queries state, no writes |
| `SafeStateProbe` | Writes only to establish safe state |
| `SafeWriteProbe` | Writes bounded operational params |
| `OperatorApprovedProbe` | Requires explicit approval |

## Public API

```rust
use odmr_preflight::{StationProfile, run_station_preflight, DeviceLock, StationLedger};

let profile = StationProfile::load("station.json")?;
let report = run_station_preflight(&profile, Some(&ledger_path), true)?;
```

## Dependencies

- `odmr-types` — device IDs and timestamps
- `serialport` — serial transport for OE, Maynuo, CNI
- `fs2` — POSIX file locks
- `cni_laser_fake_driver` — CNI laser protocol frames

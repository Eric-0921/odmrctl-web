# Mag-M0.5: Reverse-Derived Maynuo M8812 Protocol Alignment + GUI Backend Contract

## Milestone Definition

Mag-M0.5 is a **narrow alignment layer** between the mock-only Mag-M0 planning model and the reverse-engineered Maynuo M8812 control logic from `SimplePowerController.exe`. It does **not** replace Mag-M0, nor does it introduce real hardware control. Its purpose is to:

1. Document what was learned from reverse engineering.
2. Map the old diagonal axis-gain model to the general 3×3 coil matrix model.
3. Define the SCPI command vocabulary as **data-only command plans** (no serial I/O).
4. Establish the GUI → Tauri backend → executor → Maynuo driver contract for future milestones.
5. Preserve the mock-only boundary: Mag-M0.5 still cannot output real current.

## What Was Learned from Reverse Engineering

### Source Materials

| File | Purpose |
|------|---------|
| `reverse_application/reverse_output/逆向分析报告-协议与算法还原.md` | Complete reverse report: protocol, algorithm, data format |
| `reverse_application/reverse_output/verify_protocol.py` | Dynamic validation script (10mA safe current) |
| `reverse_application/reverse_output/para.xml` | Runtime config: ports, baudrate, coil constants |
| `reverse_application/reverse_output/decompiled/SimplePowerController/FormMain.cs` | Decompiled C# core (~1835 lines) |

### Key Findings

1. **No obfuscation**: Standard .NET 4.8 assembly, fully readable type/method/variable names.
2. **Standard SCPI subset**: `SYST`, `VOLT`, `CURR`, `OUTP`, `MEAS` — no custom protocol.
3. **Simple algorithm**: `Mag(nT) = Curr(mA) × CoilConstant(nT/mA)` with 5000mA hard limit.
4. **Verified on hardware**: All three axes (X/Y/Z) respond correctly to the documented sequence.

## Device Fingerprint Verification (2026-06-01)

**Method**: Power-cycle identification — send `*IDN?` to all ports, power off one axis, rescan to see which SN disappears.

**Result**: The SN→axis mapping from `odmr-types` and `para.xml` is **correct**.

| 轴 | 完整 SN (*IDN? 第3字段) | 验证方式 |
|---|------------------------|---------|
| **X** | `0800209602204020**20**` | 关闭后 `/dev/cu.PL2303G-USBtoUART11120` 无响应 |
| **Y** | `0800209602204020**22**` | 关闭后 `/dev/cu.PL2303G-USBtoUART11110` 无响应 |
| **Z** | `0800209602204020**03**` | X+Y 关闭后唯一在线设备 |

**重要**: macOS 上 `/dev/cu.PL2303G-USBtoUART111xx` 这类路径是 USB 重新枚举时**动态分配**的，每次插拔/重启后编号会变。`COM3/COM4/COM6` 是 Windows 下的记录，同样不稳定。**唯一可靠的设备标识是 SN（*IDN? 返回的第三个字段）**。

Device Registry M2 实现时必须：
1. 枚举所有串口，发送 `*IDN?`
2. 按返回的 SN 匹配到逻辑轴（X/Y/Z）
3. 绝不依赖端口路径作为绑定依据

## Maynuo M8812 Command Sequence

### Serial Parameters

| Parameter | Value | Notes |
|-----------|-------|-------|
| Baud rate | 9600 | Configurable 1200–115200 in original UI |
| Data bits | 8 | |
| Parity | None | |
| Stop bits | 1 | |
| Flow control | None | |
| DTR | true | Required for M8812 |
| Read timeout | 100 ms (code) / 300 ms (verify script) | |

### Connection Initialization

```
1. *IDN?              → verify identity (expect "MAYNUO,M8812,<SN>,V2.7")
2. SYST:REM           → remote control mode
3. VOLT 75            → set output voltage to 75V
4. CURR 0             → set output current to 0A
5. OUTP 0             → turn off output
```

### Runtime Commands

| Command | Unit | Format | Trigger |
|---------|------|--------|---------|
| `CURR {value}` | A | `CURR {:.5f}` | Current setpoint change |
| `OUTP 1` | — | `OUTP 1` | User enables output |
| `OUTP 0` | — | `OUTP 0` | User disables output |
| `MEAS:CURR?` | A (response) | `MEAS:CURR?` | Periodic poll (300ms) |
| `SYST:LOC` | — | `SYST:LOC` | Disconnect / return to local |

### Disconnection Sequence

```
1. CURR 0             → zero current
2. OUTP 0             → turn off output
3. SYST:LOC           → local mode
4. Close port
```

### Current Command Formula

```csharp
// Original C# (FormMain.cs:735-739)
string cmd = $"CURR {Math.Abs(totalCurr) / 1000.0:f5}\n";
```

- `totalCurr` is in **mA**.
- `Math.Abs()` ensures positive only (M8812 does not support negative current setpoints).
- Field direction is fixed by physical coil winding.
- Format is **5 decimal places**.

## Axis-Gain Model vs 3×3 Coil Matrix Model

### Original Software Model (Diagonal Axis-Gain)

The original `SimplePowerController` uses a per-axis gain model:

```
Bx(nT) = Ix(mA) * CoilConstantX(nT/mA)
By(nT) = Iy(mA) * CoilConstantY(nT/mA)
Bz(nT) = Iz(mA) * CoilConstantZ(nT/mA)

Ix(mA) = Bx(nT) / CoilConstantX(nT/mA)
Iy(mA) = By(nT) / CoilConstantY(nT/mA)
Iz(mA) = Bz(nT) / CoilConstantZ(nT/mA)
```

This is a **diagonal 3×3 matrix** with zero cross-coupling:

```
M = | kx  0   0  |
    | 0   ky  0  |
    | 0   0   kz |
```

where `k = CoilConstant(nT/mA) * 1e-6` to convert to T/A.

### Mag-M0 General 3×3 Coil Matrix

Mag-M0 supports the general case:

```
B = M * (I - I_offset) + B_zero_offset
```

where `M` can have off-diagonal terms for cross-coupling.

### Mapping

The Maynuo diagonal model is a **special case** of the general coil matrix:

```
M_maynuo = diag(kx_T_per_A, ky_T_per_A, kz_T_per_A)
I_offset = [0, 0, 0] A      (from para.xml: ZeroOffset = 0 mA on all axes)
B_zero_offset = [0, 0, 0] T  (no residual field documented)
```

**Conversion:**

| Direction | Formula | Example (X axis, 143.26 nT/mA) |
|-----------|---------|--------------------------------|
| nT/mA → T/A | `k_T_per_A = k_nt_per_ma * 1e-6` | 143.26 × 1e-6 = 1.4326e-4 T/A |
| mA → A | `A = mA / 1000.0` | 10 mA = 0.010 A |
| A → mA | `mA = A * 1000.0` | 0.010 A = 10 mA |
| nT → T | `T = nT * 1e-9` | 1000 nT = 1e-6 T |

## Unit Conversion Table

| Context | Internal Unit | JSON Boundary | Command Unit | Display Unit |
|---------|---------------|---------------|--------------|--------------|
| B field | T | mT (normalize to T) | — | nT (old software) |
| Current | A | mA (normalize to A) | A (5 decimal places) | mA (old software) |
| Coil constant | T/A | nT/mA (normalize to T/A) | — | nT/mA |
| Angle | rad | deg (normalize to rad) | — | deg |
| Time | ms | ms | — | ms / s |

## Zero / Recur / LockZero / Output Logic

### State Model (from FormMain.cs)

```
zeroSetCurr:   per-axis zero-field bias current [mA]
recurSetCurr:  per-axis recurrence current [mA]
recurSetMag:   per-axis recurrence magnetic field [nT]
coilConstant:  per-axis gain [nT/mA]
LockZero:      bool (per-axis toggle)
Output:        bool (per-axis toggle)
```

### Mode A: LockZero = OFF

- **Output current** = `zeroSetCurr`
- `recurSetCurr` and `recurSetMag` are **display/calculation only**.
- When `zeroSetCurr` changes and `Output = ON` → send `CURR` with new value.

### Mode B: LockZero = ON + Output = ON

- **Output current** = `zeroSetCurr + recurSetCurr`
- When `recurSetCurr` changes → auto-recalculate and send `CURR`.
- When `recurSetMag` changes → convert to current (`curr = mag / coilConstant`), update `recurSetCurr`, then send `CURR`.
- When `zeroSetCurr` changes → recalculate total and send `CURR`.

### Safety Limit (from original software)

```
POWER_MAX_CURR = 5000.0 mA

If zeroSetCurr + recurSetCurr > 5000 mA:
    → Show warning "总电流超范围！"
    → Auto-limit recurSetCurr = 5000 - zeroSetCurr
    → Send CURR 5.00000
```

**Mag-M0.5 policy**: The 5000 mA hardware limit is documented but is **not** the default bring-up test current. The only low-current micro-test example permitted is **10 mA**.

## Mapping to `odmr-mag`

### New Types Added in Mag-M0.5

| Type | Purpose |
|------|---------|
| `MaynuoAxisProfile` | Per-axis config: port, SN, coil constant, serial settings |
| `MaynuoSerialSettings` | Baudrate, data bits, parity, stop bits, flow control, DTR |
| `MaynuoAxesProfile` | Three-axis assembly: X/Y/Z profiles + safety policy ref |
| `MaynuoCommand` | Enum of all SCPI commands as data (no I/O) |
| `MaynuoCommandPlan` | Ordered list of commands + metadata for a procedure |

### Command String Generation (pure data)

```rust
format_current_command_from_ma(10.0) -> "CURR 0.01000"
format_current_command_from_ma(0.0)  -> "CURR 0.00000"
```

These functions return `String`, not `Result` with serial port side effects.

### Plan Builders (pure data)

```rust
build_safe_init_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan
build_query_current_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan
build_10ma_microtest_plan(axis: &MaynuoAxisProfile) -> MaynuoCommandPlan
```

Each returns an ordered list of `MaynuoCommand` variants with expected response shapes and event names.

### Calibration Bridging

A `MaynuoAxesProfile` can be converted to a `CoilMatrix`:

```rust
impl From<&MaynuoAxesProfile> for CoilMatrix {
    fn from(profile: &MaynuoAxesProfile) -> Self {
        // Diagonal matrix from per-axis coil constants
        // kx_T_per_A = profile.x.coil_constant_nt_per_ma * 1e-6
        // ...
    }
}
```

This allows the existing `MockMagAxes`, safety checks, and planner to work with Maynuo-derived calibration without code duplication.

## Mapping to Future Executor Steps

### Mag-M2A: Identity-Only Bring-Up

The executor will use `odmr-mag` plan builders to generate dry-run verification plans:

```
Executor Phase M2A:
  1. Device registry leases Maynuo M8812 (3x)
  2. For each axis:
       a. Open serial (odmr-maynuo-m8812 driver, NOT odmr-mag)
       b. Send *IDN?
       c. Verify SN tail matches profile
       d. Record identity in station_snapshot.json
  3. Close all ports
  4. No current output, no CURR commands
```

`odmr-mag` provides the **expected command sequence** as reference data; the actual serial I/O lives in a future `odmr-maynuo-m8812` driver crate.

### Mag-M2B: Safe Initialization

```
Executor Phase M2B:
  1. Lease devices
  2. For each axis:
       a. Send SYST:REM
       b. Send VOLT 75
       c. Send CURR 0
       d. Send OUTP 0
       e. Send MEAS:CURR? (verify near-zero)
  3. Record init events in events.jsonl
  4. All commands generated from odmr-mag plan builders
```

### Mag-M3: Low-Current Micro-Test (10 mA)

```
Executor Phase M3:
  1. Safety gate validates 10 mA < policy limit
  2. For target axis:
       a. CURR 0.01000
       b. OUTP 1
       c. Wait settle_ms
       d. MEAS:CURR? (verify ~10 mA)
       e. OUTP 0
       f. CURR 0
       g. SYST:LOC
  3. Record all events
  4. Compare measured vs expected
```

## Mapping to Tauri Backend and React GUI

### Data Flow

```
React GUI (TypeScript)
  ├─ user clicks "Magnetic Planner" nav item
  ├─ renders static mock data (Mag-M0.5)
  └─ all real-control buttons are DISABLED with reason tooltip
         ↓
Tauri Command Boundary (Rust)
  ├─ magnetic_preview_plan(plan_json) → preview_json
  ├─ magnetic_generate_block(plan_json) → block_json
  └─ magnetic_validate_block(block_json) → safety_report_json
         ↓
odmr-mag (Rust, pure computation)
  ├─ plan builders → command plans
  ├─ coil matrix → current computation
  ├─ safety policy → rejection/acceptance
  └─ MockMagAxes → event timeline
         ↓
Event Logger (Rust)
  └─ writes magnetic_events.jsonl, magnetic_state_timeline.jsonl
         ↓
Tauri Event / Channel
  └─ pushes structured DTO to frontend
         ↓
React GUI displays
  ├─ B-field preview
  ├─ Coil current preview
  ├─ Command plan preview (read-only SCPI strings)
  ├─ Safety status
  ├─ Mock event timeline
  └─ Disabled controls with "requires Mag-M2 backend" tooltip
```

### GUI Contract (Mag-M0.5 Mock Display)

The GUI displays axis cards with these fields:

| Field | Source | M0.5 Value |
|-------|--------|------------|
| Axis ID | Profile | "mag_x", "mag_y", "mag_z" |
| Identity | Profile + mock | "MAYNUO,M8812,080020960220402020,V2.7" (placeholder) |
| Connection state | Mock | "disconnected_mock" |
| Output state | Mock | "off" |
| Target current | Mock / plan | From command plan preview |
| Measured current | Mock | 0.00 mA |
| Target field | Mock / plan | Computed from current × coil constant |
| Calibration summary | Profile | Coil constant, verified flag, date |
| Command preview | Plan builder | List of SCPI strings (read-only) |
| Last event | Mock timeline | "RampRejected: current_limit_exceeded" etc. |

### Disabled Controls

| Control | Disabled Reason (M0.5) |
|---------|------------------------|
| Connect | "Requires Mag-M2A identity bring-up" |
| Output ON/OFF | "Requires Mag-M2B safe init" |
| Set Current | "Requires Mag-M3 micro-test path" |
| Lock Zero | "Requires Mag-M2B backend" |
| Zero Set Current | "Requires Mag-M2B backend" |
| Recur Set Current | "Requires Mag-M2B backend" |
| Recur Set Mag | "Requires Mag-M2B backend" |
| Emergency Stop | "Visual-only in M0.5; no hardware authority" |

## What Remains Mock-Only

Everything in Mag-M0.5 is still mock-only:

- `odmr-mag` has **no serialport dependency**.
- `MaynuoCommand` variants are **data**, not function calls.
- Command plans are **JSON-serializable artifacts**.
- The GUI shows **disabled controls** with explanatory tooltips.
- No `Box<dyn SerialPort>`, no `sp.write()`, no `sp.read()`.

## What Is Postponed to Mag-M2 / Mag-M3

| Feature | Postponed To | Reason |
|---------|--------------|--------|
| Real serial port open/close | Mag-M2A | Requires `serialport` crate + device registry |
| Real `*IDN?` query | Mag-M2A | Requires hardware connection |
| Real `CURR` command send | Mag-M2B | Requires init verification + safety gate |
| Real `OUTP 1` | Mag-M3 | Requires 10mA micro-test safety review |
| Real `MEAS:CURR?` readback | Mag-M3 | Requires active output for meaningful data |
| GUI enable real controls | Mag-M3 | Requires full backend + executor + safety chain |
| Auto-limit at 5000mA | Mag-M2B | Software-side safety; hardware has its own limit |
| LockZero / zero+recur logic | Mag-M3 | Requires real output to be meaningful |
| Data save (CSV/txt) | Mag-M3+ | Out of scope; real-time is raw bin only (ADR-005) |
| Cross-axis coupling calibration | Mag-M4+ | Requires field probe measurement for off-diagonal terms |

## Compliance with ADR-008

Mag-M0.5 does not weaken ADR-008:

- `cargo tree -p odmr-mag` still shows no `serialport`, `rusb`, `hidapi`, `visa-rs`.
- `grep` for hardware keywords in `crates/odmr-mag/src/` still returns empty.
- `MaynuoCommand` is an enum of strings, not a trait over serial ports.
- Plan builders return data structures, not I/O futures.
- The 5000 mA hardware limit is documented but not treated as a default test current.
- The only executable micro-test example is 10 mA.

## Files Added in Mag-M0.5

| File | Purpose |
|------|---------|
| `docs/lab-bringup/mag_m0_5_reverse_protocol_alignment.md` | This document |
| `examples/magnetic/maynuo_m8812_axes.example.json` | Three-axis profile with coil constants |
| `examples/magnetic/maynuo_m8812_safe_init_plan.example.json` | Dry-run safe init command plan |
| `examples/magnetic/maynuo_m8812_10ma_microtest_plan.example.json` | Future 10mA micro-test plan |
| `examples/magnetic/maynuo_m8812_gui_contract.example.json` | GUI/backend data shape contract |
| `crates/odmr-mag/src/lib.rs` (additions) | Maynuo types, command builders, tests |

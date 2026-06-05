# 设备操控参数覆盖度审计

> 本文件由用户对话整理生成，目的：对照面板截图 JSON、SCPI 设备手册、当前 Rust 命令目录
> 三方资料，识别我们与原始设计目标（"通过 JSON 配置设备参数"）之间的差距。
> **不修改任何代码。**
>
> **更新**：2026-06-05 包含 `2984998` (M5B-B JSON config model) 和 `d210e3d` (command catalog gaps)
> 两次大提交后的最新状态。

---

## 0. 用户原始设想

> "不用在具体的做某个设备的 GUI 面板，我们可以直接通过 json 文件配置设备的参数，
> 或者说类似步进的 sweep 参数，某些不需要变更的配置，我可以快捷选择覆盖那一列的配置。
> 有些设置需要长期的维持，有些参数需要和其他设备一起联动，每一步都有所改变这样子。"

→ 期望形态：

| 类型 | 例子 | JSON 表达 |
|------|------|----------|
| 静态配置（不扫动） | SMB OE1022D sensitivity | 直接列在 profile 里 |
| 长期保持的联动设置 | SMB freq + RF ON 一起设定 | 同 step 内多 device 协同 |
| 步进 sweep | SMB frequency 2820→2920 MHz step 5 MHz | 已有 `sweeps` 数组 |
| 增量覆盖 | 整组配置覆盖某列某字段 | 需要 patch / override 语义 |
| step-by-step 改变 | 每一步都改一组参数 | recipe 已有 `blocks` + `sweeps` |

---

## 1. SMB100A 覆盖度

### 1.1 examples JSON 已识别的设置（来自 6 张图）

**主屏顶部 5 个常量：**
- `rf_frequency` (FREQuency:CW)
- `rf_output_state` (OUTPut:STATe)
- `modulation_global_state` (MODulation:ALL:STATe)
- `rf_level` (POWer:LEVel:IMMediate:AMPLitude)
- `alc_state` (POWer:ALC:STATe)
- `lf_output_state` (LFOutput:STATe) — Mod Gen 块

**LF Generator / Output（图3）：**
- LF Frequency (LFO:FREQ)
- LF Voltage (LFO:VOLT) — peak 电压
- LF Shape (LFO:SHAP) — SINE/SQUARE/TRIangle/RAMP
- Source Impedance (LFO:SIMP) — LOW / G600

**Modulation → Frequency Modulation（图5/6）：**
- FM State (FM:STATe)
- FM Source (FM:SOURce) — INTernal/EXTernal/INT+EXT
- FM Mode (FM:MODE) — NORMal/HDEViation/LNOise
- FM Deviation (FM:DEViation)
- FM Sensitivity (FM:SENSitivity?) — 仅 Ext 时
- FM Ext Coupling (FM:EXTernal:COUPling) — AC/DC
- FM Ext Impedance (INPut:MODE:IMPedance) — HIGH/G600
- Ignore Overvoltage Warning (INPut:MODE:WIGNore) — ON/OFF
- Adjust FM Offset (CALibration:FMOFfset:MEASure?)

### 1.2 SCPI 手册（SOURce 子系统）全部列出的设置

`docs/equipment_manual/smb100a/06l_source_subsystem.md` 共 **~5716 行**，

SOURce 子系统清单：

```
6.13.1  SOURce:AM       — 振幅调制      [完全缺失]
6.13.2  SOURce:CORRection — 频响/功率修正  [完全缺失]
6.13.3  SOURce:FM        — 频率调制      [部分]
6.13.4  SOURce:FREQuency — 频率/扫频     [部分]
6.13.5  SOURce:INPut     — 外部调制输入   [缺失]
6.13.6  SOURce:LFOutput  — LF 发生器     [部分]
6.13.7  SOURce:LIST      — List 模式     [完全缺失]
6.13.8  SOURce:MODulation — 总调制开关   [已有]
6.13.9  SOURce:PGENerator — 脉冲发生器   [完全缺失]
6.13.10 SOURce:PHASe     — 相位控制      [完全缺失]
6.13.11 SOURce:PM        — 相位调制      [完全缺失]
6.13.12 SOURce:POWer     — 功率/ALC      [部分]
6.13.13 SOURce:PULSe     — 脉冲调制      [完全缺失]
6.13.14 SOURce:ROSCillator — 参考振荡器  [完全缺失]
6.13.15 SOURce:SWEep     — 扫频扫幅     [部分]
6.13.16 SOURce:DM        — 失真/调制     [完全缺失]
6.13.17 SOURce:IQ        — IQ 调制       [完全缺失]
6.13.18 SOURce:NOISe     — 噪声注入     [完全缺失]
```

### 1.3 当前 `crates/odmr-smb100a/src/commands.rs` 实现（最新 ~32 个函数）

`d210e3d` 提交新增 2 个：`set_lf_impedance` / `set_fm_mode`，
`3c3c955` 提交修正了 LFO:SIMP / FM:MODE 的合法枚举。

已有：
- FREQuency: CW / STARt / STOP / STEP / MODE
- POWer: LEVel / ALC
- OUTPut: STATe
- MODulation: ALL STATe
- LFOutput: STATe / FREQuency / VOLTage / SHAPe / SIMPedance
- FM: STATe / SOURce / MODE / DEViation
- SWEep: STEP / DWELl / SPACing / MODE

**枚举修正后正确的合法值（来自 `3c3c955` 注释）：**
- `LFO:SIMP`: `LOW` / `G600`（原错误：`HIGH` / `G50`）
- `FM:MODE`: `NORMal` / `LNOise` / `HDEViation`（原错误：把 LNOise 写成了 LDEV）

**对 ODMR 关键但仍完全缺失的：**

| 子系统 | 关键 SCPI | ODMR 实验用途 | 当前 JSON profile 表达 |
|--------|-----------|----------------|------------------------|
| AM 调制 | `AM:STATe` `AM:SOURce` `AM:DEPTh` `AM:TYPE` | 振幅 ODMR | ❌ 无 |
| PM 相位调制 | `PM:STATe` `PM:DEViation` `PM:SOURce` | 与 FM 切换 | ❌ 无 |
| **PULSe 脉冲调制** | `PULM:STATe` `PULM:SOURce` `PULM:TTYPe` | **NV Rabi 序列** | ❌ 无 |
| **PGEN 脉冲发生器** | `PGENerator:STATe` `PGENerator:PERiod` `PGENerator:DOUBle:PULSe` | **脉冲链控制** | ❌ 无 |
| PHASe 连续相位 | `PHASe` `PHASe:REFerence` | 相位连续扫描 | ❌ 无 |
| ROSCillator 参考源 | `ROSCillator:SOURce` `ROSCillator:EXTernal:FREQuency` | 时基切换 | ❌ 无 |
| LFOutput:SWEep 完整 | `LFOutput:SWEep:FREQuency:MODE` `STEP:LINear` `STEP:LOGarithmic` `DWELl` `RETRace` | LF 扫频 | ❌ 无 |
| **LIST 模式** | `LIST:CATalog?` `LIST:DWELl` `LIST:FREQuency` `LIST:LEVel` `LIST:MODE` | **离散频率/功率序列** | ❌ 无（commit message 明确说"未来优化，未实现"） |
| CORRection 频响修正 | `CORRection:CSET:...` `CORRection:STATe` | 现场校准 | ❌ 无 |
| Power 高级 | `POWer:ATTenuation:RFOFf:MODE` `POWer:ALC:OMODe` | 衰减器/ALC 源 | ❌ 无 |
| FREQuency 高级 | `FREQuency:CENTer` `FREQuency:SPAN` `FREQuency:MULTIPlier` | 中心+跨距扫频 | ❌ 无 |
| SWEep 高级 | `SWEep:RF:MODE` `SWEep:POWer:MODE` `SWEep:EXECute` | 真正触发扫频 | ❌ 无 |

**注**：完整 SCPI 列表见 `docs/equipment_manual/smb100a/06l_source_subsystem.md`
（共 16+ 个子系统，从 AM 到 ROSCillator 到 SWEep）。

### 1.4 JSON profile 表达与 SCPI 之间的桥（`d210e3d` 后）

`examples/device_profiles/smb100a.full.profile.json` 已经把 commands.rs
能覆盖的子集表达为强类型字段：

```json
{
  "rf": { "frequency_hz", "power_dbm", "output_enabled", "frequency_mode", "alc_state" },
  "modulation": { "global_enabled" },
  "fm": { "enabled", "source", "mode", "deviation_hz" },
  "lf": { "output_enabled", "frequency_hz", "shape", "voltage_v", "source_impedance" }
}
```

这 14 个字段全部对应 commands.rs 已有的 ~14 个 setter。
**比例：JSON 表达覆盖 14/200+ 个 SCPI 概念 ≈ 7%。**

**对比 M5B-B commit message 描述与现实的缺口：**
- M5B-B 强类型化的是"基础 RF/FM/LF 静态配置"
- 完全没有覆盖 `PULSe` / `PGEN` / `LIST` / `SWEep:EXECute` / `CORRection` / `AM` / `PM` / `PHASe` 等 ODMR 真正需要的子系统

**SMB100A SCPI 共 ~180+ 个；当前 ~32/180 ≈ 18% 覆盖**（相对之前的 5% 已经翻 3 倍）。

---

## 2. OE1022D 覆盖度

### 2.1 examples JSON 已识别的设置（5 个 LabVIEW 面板）

| 面板 | 参数数 | 代表命令 |
|------|-------|---------|
| Channel Out + Sine Out | 13 | SWVTD/SLVLD/SVLLD/SVULD/SVSLD/SVSGD/SVTMD/SVRMD/SVDCD/FPOPD/OEXPD/SPEDD/CAUXD |
| Formula System | 4 | EQCDD/EQCSD/SSETD/RSETD |
| Input Filter | 15 | ISRCD/IGNDD/ICPLD/ILIND/RMODD/SENSD/AGAND/ARSVD/OFLTD/OFSLD/SYNCD/HARMD(2)/ASCLD |
| Reference Signal | 12 | FMODD/RSLPD/FREQD/PHASD/APHSD/SWTPD/SLLMD/SULMD/SSLLD/SSLGD/STLMD/SWRMD |

合计 **44 个 LabVIEW 参数 → 30+ 个 SCPI 命令**。

### 2.2 设备手册（`05_oe1022d_remote_programming_commands_55_74.md`）全部命令

5.2.1 参考与相位（已纳入）
5.2.2 公式系统（已纳入）
5.2.3 输入与滤波器（已纳入）
5.2.4 灵敏度/动态储备/滤波器时间常数（已纳入）
5.2.5 显示和输出（已纳入）
5.2.6 保存读取（已纳入）
5.2.7 自动设置（已纳入）
5.2.8 数据存储 ⚠️ **新发现**
5.2.9 数据读取 ⚠️ **新发现**
5.2.10 复位与 IDN

### 2.3 `crates/odmr-oe1022d/src/commands.rs` 实现（`d210e3d` 后约 60+ 函数）

**最新进展**：`d210e3d` 提交一次性补齐 29 个缺失命令 + 21 个 golden 测试 + 4 个
fake-device 测试。具体新增：

| 子系统 | 新增 | 数量 |
|--------|------|------|
| Sine Output | SWVTD, SLVLD, SVLLD, SVULD, SVSLD, SVSGD, SVTMD, SVRMD, SVDCD | 9 |
| Channel Output | FPOPD, OEXPD, SPEDD, CAUXD | 4 |
| Reference Sweep | SWTPD, SLLMD, SULMD, SSLLD, SSLGD, STLMD, SWRMD | 7 |
| Auto Settings | AGAND, ARSVD, APHSD, ASCLD | 4（**`d210e3d` commit message 写 5 个，实际 4 个 — APHSD/APOVD 算一个**）|
| Equation System | EQCDD, EQCSD | 2 |
| Save/Recall | SSETD, RSETD | 2 |

**仍未实现的（5.2.8/5.2.9 数据子系统）：**

| 命令 | 用途 | 状态 |
|------|------|------|
| `SSLED` | 数据采样使能/配置 | ❌ |
| `OUTPD` | 单点输出读取 | ❌ |
| `SNAPD` | 快照式多参数读取 | ❌ |
| `OAUXD` | AUX 输入读取 | ❌ |
| `SPTSD` | 采样点设置 | ❌ |
| `TRCAD` | trace 数据读取 | ❌ |
| `INOVD` / `GNOVD` | 输入/增益过载状态 | ❌ |
| `*PLLD?` | PLL 锁定状态 | ❌ |

注：`INOVD` / `GNOVD` / `*PLLD?` 在老的 `commands.rs` 中以 `query_input_overload` /
`query_gain_overload` / `query_pll_locked` 形式存在，但没构造成完整子系统的 setter。

### 2.4 RALL? 采集面板

`docs/equipment_manual/oe1022d/05_oe1022d_rall_global_data_config_reading.md`
（标题暗示还有一整套"全局数据配置读取"相关 SCPI）

OE1022D 设备上还要考虑：
- `SYNCD` 同步滤波器谐波 ✅（已加入 commands.rs）
- `HARMD` 谐波 1/2（与基频/2f/3f 实验有关）✅
- `ASCLD` 自动量程 ✅
- RALL? 帧内全部字段（已在 `parser.rs` 处理）
- **RALL? 全局配置** ⚠️ `commands.rs` 没有 `read_all` 的参数化版本
  之外的配置能力（如 `SSLED` 启用/配置采样使能、采样点设置 `SPTSD`）

**JSON profile 表达**（`examples/device_profiles/oe1022d.full.profile.json`）：

```json
{
  "primary_channel", "primary_value",
  "input": { "source", "shield_grounding", "coupling", "notch_filter" },
  "reference": { "source", "external_trigger", "phase_deg", "auto_phase" },
  "gain": { "dynamic_reserve", "sensitivity" },
  "filter": { "time_constant_s", "slope_db_oct", "sync_filter_enabled" },
  "harmonic": { "harmonic_1", "harmonic_2" },
  "acquisition": { "frames_per_point", "inter_frame_delay_ms", "pre_discard_ms", "record_fields" }
}
```

约 22 个 JSON 字段，对应 commands.rs 已有 setter。
**比例：22/60+ ≈ 35% 覆盖**（已较好，仍缺数据采集 5.2.8/5.2.9 子系统）。

---

## 3. Maynuo M8812 覆盖度

### 3.1 真实面板参数（来自反向工程）
`docs/prd/04_recipe_json_schema_prd_v0.2.md` + `reverse_application/`

- 电流值
- 电压量程
- 输出开/关
- 本地/远程模式（SYST:LOC / SYST:REM）
- 电流/电压上限设置
- 触发模式

### 3.2 当前 `crates/odmr-maynuo-m8812/src/lib.rs`（`2984998` 后）

**`2984998` (M5B-B) 提交内容**：
- 在 doc 注释中加入 "Mag-M2C : `SYST:ERR?`, `VOLT:PROT`"
- 在 `is_allowed()` allowlist 加入 `"SYST:ERR?"` 和 `"VOLT:PROT 75"`
- 给 `MaynuoM8812Transport` 新增 `query_error()` 方法
- 给 `send_set_voltage()` 支持 VOLT:PROT 协议

修正了**最大电流 5000mA → 2000mA**（M8811 spec 错填成 M8812 的 0-2A 真实规格）。

**仍只有 4 个 pub 类型**（仍然是配置 + 传输 + 错误结构，没有 SCPI 字符串构造器 pub fn）：
- `MaynuoSerialPortConfig`
- `MaynuoPortMetadata`
- `MaynuoProbeError`
- `MaynuoM8812Transport`

**该 crate 当前提供的方法**（impl 块内）：
- `enumerate_ports` / `open` / `port_path` / `config`（基础设施）
- `query_idn` / `query_meas_current` / `query_error`（query）
- `send_set_remote` / `send_set_local`（SYST:REM / SYST:LOC）
- `send_set_voltage`（VOLT 75 / VOLT:PROT 75）
- `send_set_current`（CURR x.xxxxx）
- `send_set_output`（OUTP 0/1）

**架构偏差仍未纠正**：
- `odmr-mag/src/lib.rs` 仍保留 `format!("CURR {x:.5}")` /
  `format!("VOLT {n}")` / `format!("VOLT:PROT {n}")` / `format!("OUTP {}")` 的
  直接命令构造（`MaynuoCommand::to_scpi()` 等 ~10 处）
- 应该有 `pub fn set_current_ma(ma: f64) -> String` 等独立构造器放在 maynuo crate

### 3.3 命令覆盖度（`2984998` 后）

`docs/equipment_manual/maynuo_dc-power-supply/m8812_remote_control_reference.md`
（也是 `2984998` 提交新增的，从 m88 手册中提取的专用参考）列出全部 22 个 SCPI：

| SCPI | 用途 | maynuo crate | mag crate | status |
|------|------|-------------|-----------|--------|
| `*IDN?` | 身份 | ✅ `query_idn` | ❌ | OK |
| `SYST:REM` | 远程模式 | ✅ `send_set_remote` | ✅ | OK |
| `SYST:LOC` | 本地模式 | ✅ `send_set_local` | ✅ | OK |
| `SYST:ERR?` | 错误队列 | ✅ `query_error` (M2C) | ❌ | **M5B-B 新增** |
| `SYST:SENS <bool>` | 远程 sense | ❌ | ❌ | 缺 |
| `MEAS:CURR?` | 测电流 | ✅ `query_meas_current` | ✅ | OK |
| `MEAS:VOLT?` | 测电压 | ❌ | ❌ | **缺** |
| `MEAS:DVM?` | 内置 DVM | ❌ | ❌ | 缺 |
| `VOLT <val>` | 电压上限 | ✅ `send_set_voltage(75)` | ✅ | OK（仅 75） |
| `VOLT:PROT <val>` | 过压保护 | ✅ `send_set_voltage` (M2C) | ✅ | **M5B-B 新增** |
| `CURR <val>` | 电流值 | ✅ `send_set_current` | ✅ | OK |
| `OUTP <bool>` | 输出开关 | ✅ `send_set_output` | ✅ | OK |
| `CURR?` | 读程序电流 | ❌ | ❌ | **缺** |
| `VOLT?` | 读程序电压 | ❌ | ❌ | **缺** |
| `OUTP?` | 读输出状态 | ❌ | ❌ | **缺** |
| `LIST:*` | 列表模式 | ❌ | ❌ | 缺（暂未做，commit 明确"未来优化"） |
| `*RST` | 复位 | ❌（应永久禁用） | ❌ | 应当 allowlist 拒绝 |
| `*CLS` | 清状态 | ❌ | ❌ | 缺 |
| `*OPC?` | 操作完成 | ❌ | ❌ | 缺 |

**当前覆盖 ~9/22 ≈ 41%**（相较之前的 0% 大幅提升，但仍有 13 个缺）。

---

## 4. 磁场（Mag / Maynuo 联动）

### 4.1 `crates/odmr-mag/src/lib.rs` 已实现（~50 pub 项）

- 三轴坐标转换（笛卡尔/球坐标）
- nT ↔ mA 换算
- 各种 plan 构造（`build_safe_init_plan` 等 ~15 个）
- 零位锁定流程
- 周期 microtest
- **coil_matrix 求逆**（`2984998` 新增）→ 用于 predicted_current 计算

### 4.2 关键改动：`2984998` 引入 `system_scan` 概念

`crates/odmr-recipe/src/system_scan.rs`（96 行）+ `crates/odmr-compiler/src/system_scan.rs`（319 行）
+ `crates/odmr-safety/src/system_scan.rs`（1064 行）三个新模块。

**新增 device_params.rs（1027 行）** 强类型化四类设备配置：
- `Smb100aConfig`（含 `RfConfig` / `ModulationConfig` / `FmConfig` / `LfConfig`）
- `Oe1022dConfig`（含 `InputConfig` / `ReferenceConfig` / `GainConfig` / `FilterConfig` / `HarmonicConfig` / `AcquisitionConfig`）
- `MagneticConfig`（含 `AxesConfig` / `AxisConfig` / `CoilMatrix` / `ZeroOffsets`）
- `LaserConfig`
- `StationSafety`（含 `Smb100aSafetyLimits` / `Oe1022dSafetyLimits` / `MagneticSafetyLimits` / `LaserSafetyLimits`）

**safety 现在从 station 拉 limits**（`2984998` Phase 1-3 之一），
不再硬编码 — 移除了原本的"魔法数"。

### 4.3 缺失/未充分表达

- **与 SMB 的同步 sweep**：`m5b_rf_mag_oe_system_scan.recipe.json` 现在通过
  `sweeps` 数组 + `sweep_order` 隐式实现 RF×Mag 笛卡尔积，但没有
  `"linked": "outer_product | zip | expr"` 的显式表达
- **多轴联动**：B 矢量 cartesian_grid 已支持（`"type": "cartesian_grid"`），
  但还**没有 spherical sweep**（极轴 + 仰角）
- **路径 sweep**：`B = B0 + α·dir` 形式未在 schema 中表达
- **磁场 RSS 安全限**：仍是单轴电流限，**没实现** `B_x² + B_y² + B_z² < B_max²` 的矢量安全检查
- **settle_ms / ramp_ms**：`MagneticConfig.default_settle_ms` 已存在，但
  per-step override 机制尚未实现

---

## 5. 配方 JSON 形态的偏差

### 5.1 现在有两种 Recipe JSON（`2984998` 后）

**类型 A：传统 ODMR recipe**（`examples/recipes/basic_odmr_mock.recipe.json`）：

```json
{
  "schema_version": "0.2.0",
  "kind": "recipe",
  "profiles": ["smb100a_default_fm_500hz", ...],
  "blocks": ["block_smb_fm_500hz_4mhz"],
  "sweeps": [{"axis": "smb100a.rf.frequency_hz", "start": ..., "stop": ..., "step": ...}],
  "acquisition": {"device_id": "oe1022d_01", "channel": "B", ...}
}
```

**类型 B：system_scan_recipe**（`examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json`，`2984998` 新增）：

```json
{
  "schema_version": "0.2.0",
  "kind": "system_scan_recipe",
  "station_ref": "...",
  "devices": { "smb100a": { "device_id", "required" }, ... },
  "fixed_params": {
    "smb100a": { "rf": {...}, "fm": {...}, "lf": {...} },
    "oe1022d": { "input": {...}, "gain": {...}, "filter": {...}, "harmonic": {...}, "acquisition": {...} },
    "magnetic": { "axes": {...}, "coil_matrix": {...}, "default_settle_ms": 500, ... },
    "laser": { "enabled": false, "power_mw": 0.0, "wavelength_nm": 532.0, ... }
  },
  "sweeps": [
    { "sweep_id": "mag_z_low_current_points", "device": "magnetic", "type": "cartesian_grid",
      "axes": { "bx_nt": {"value": 0.0}, "by_nt": {"value": 0.0}, "bz_nt": {"values": [-1000.0, 0.0, 1000.0]} } },
    { "sweep_id": "rf_frequency_points", "device": "smb100a", "axis": "smb100a.rf.frequency_hz",
      "values": [2878000000, 2882000000, 2886000000] }
  ],
  "sweep_order": ["mag_z_low_current_points", "rf_frequency_points"],
  "acquisition_policy": { "enabled": true, "device": "oe1022d", "mode": "per_final_sweep_point",
    "start_after": ["magnetic_settled", "rf_configured", "rf_output_on_confirmed"],
    "pre_discard_ms": 100, "frames_per_point": 5, "attach_device_state_snapshot": true },
  "safety": { "require_operator_approval": true, "no_internal_smb_sweep": true, ... }
}
```

**重大改进**（`2984998` 后）：
- `kind: "system_scan_recipe"` 是新分支（独立于 "recipe"）
- 引入 **`fixed_params`** 段：完整的设备配置快照（不是只引 profile 名）
- 引入 **`sweep_order`**：明确多 sweep 的笛卡尔积顺序
- 引入 **`type: "cartesian_grid"`** + `axes` 子结构：支持笛卡尔网格 sweep
- 引入 **`acquisition_policy.mode: "per_final_sweep_point"`** + `start_after` 事件门控
- 引入 **`safety` 子句**：`no_internal_smb_sweep` / `no_realtime_csv` / `no_gui_direct_hardware` / `laser_default_disabled`

### 5.2 用户的"快捷覆盖一列"诉求

`fixed_params` + `sweeps` 的组合已经部分支持"覆盖一列"：
- 全局：`fixed_params.smb100a.rf.frequency_hz = 2882e6`（默认）
- 扫这个 sweep 内的点：`sweeps[?].values = [2878, 2882, 2886]`
- → 实际效果 = 在 fixed 默认值覆盖这个 sweep 域

**但没有 step-by-step 条件覆盖**，比如：
- "step 5 把 SMB 频率从 2.87 改成 2.89"
- "step 7 改 sensitivity"

→ 仍需建一个完整新 sweep 数组，没有 step 索引级别的 patch。

### 5.3 用户的"长期保持"诉求

`fixed_params` **就是"长期保持"层**：
- `fixed_params.smb100a.fm` 5 个字段 → 每步都设
- `fixed_params.oe1022d.gain` → 每步都设
- `fixed_params.magnetic.coil_matrix` → 实验标定后长期不变

**这部分已经满足用户需求。**

### 5.4 用户的"每步都改变"诉求

`sweeps` 数组已经处理"每步都变"：
- 单设备单轴：`{"axis": "smb100a.rf.frequency_hz", "values": [...]}`
- 多设备笛卡尔积：通过 `sweep_order` 数组隐式定义
- 多轴矢量网格：`{"type": "cartesian_grid", "axes": {"bx_nt": {...}, "by_nt": {...}}}`

**仍未显式表达的**：
- `linked: "outer_product" | "zip" | "expr"`（笛卡尔积 / 一一配对 / 表达式）
- 球坐标 sweep（spherical_grid / path）
- step 级别 patch override

---

## 6. 总结：差距清单（按优先级）— `2984998` + `d210e3d` 之后

### 重大改进

- ✅ **OE1022D +29 命令**（`d210e3d`）：sine output / channel output / reference sweep / auto settings / equation / save-recall 全套
- ✅ **M5B-B JSON config model**（`2984998`）：强类型化 `device_params.rs`（1027 行）
  + `system_scan_recipe` 新分支 + 4 个 full profile JSON + 1 个 full station JSON
  + 17 个新测试
- ✅ **Maynuo 新增 `SYST:ERR?` 和 `VOLT:PROT`**（`2984998`）
- ✅ **safety 用 StationSafety limits**（`2984998`）替代硬编码
- ✅ **coil_matrix 求逆**（`2984998`）

### 仍存在的差距

#### P0 — 必须有（实验基础）

1. **SMB100A 仍缺 ~80% SCPI**：~32/180+ 命令
   - **P0 子项（commit `2984998` 明确说"未实现"）**：
     - `PULSe` / `PGENerator`（NV Rabi 序列基础）
     - `LIST` 模式（commit message 明确说"未来优化，未实现"）
     - `SWEep:EXECute`（真触发扫频）
     - `CORRection`（现场校准）
2. **OE1022D 数据采样/读取子系统**（5.2.8/5.2.9）：`SSLED` / `OUTPD` / `SNAPD` / `OAUXD` / `SPTSD` / `TRCAD` 全缺
3. **Maynuo 仍缺 ~59% 命令**：`MEAS:VOLT?` / `MEAS:DVM?` / `*RST` 拒绝 / `*CLS` / `*OPC?` / `CURR?` / `VOLT?` / `OUTP?` / `SYST:SENS`
4. **架构偏差**：maynuo 命令构造器仍散落在 `odmr-mag`（`format!` 散落 ~10 处）
5. **recipe JSON 缺 step 级 patch override**（用户的"快捷覆盖一列"还没完全实现）

#### P1 — 应该有（实验常用）

6. SMB100A `AM` `PM` `PHASe` `ROSCillator`（多调制源切换）
7. SMB100A `LFOutput:SWEep` 完整子系统
8. SMB100A `POWer:ATTenuation` / `ALC:OMODe` / `ALC:SONCe`（高功率安全）
9. **磁场球坐标 sweep**（spherical_grid）+ **路径 sweep**（path / linear interpolation）
10. **磁场 RSS 矢量安全限**（`B_x² + B_y² + B_z² < B_max²`）
11. recipe `linked: "outer_product | zip | expr"` 显式声明

#### P2 — 可以晚做

12. SMB100A `IQ` / `NOISe` / `DM`（暂时用不到）
13. SMB100A `CORRection:DEXChange`（频响文件导入 — 暂时手算）
14. Maynuo `LIST` 模式
15. CNI Laser 真实 driver（fake only）
16. trace service 实时降采样

---

## 7. 建议的"我们想要的样子"— 对照 `2984998` 已有 vs 仍需补充

**`2984998` 后的 system_scan_recipe 已经实现了我们之前设想的大部分：**

| 用户诉求 | `2984998` 已实现 | 仍需补充 |
|----------|------------------|---------|
| 长期维持配置 | `fixed_params` 段 | 命名建议改为 `persistent` 更直白 |
| 步进 sweep | `sweeps` 数组 + `sweep_order` | 已支持笛卡尔网格，**缺** `linked` 显式语义 |
| 多设备协调 | `sweep_order` 隐式定义 | 应升级为 `linked: "outer_product | zip | expr"` |
| 步级条件覆盖 | **缺** | 应新增 `per_step_overrides: [...]` |
| 完整设备配置快照 | `fixed_params.{smb100a,oe1022d,magnetic,laser}` 强类型 | 已 OK |
| 矢量 sweep | `cartesian_grid` | **缺** `spherical_grid` / `path` |
| acquisition_policy 事件门控 | `start_after: [...]` 数组 | 已 OK |
| 显式 safety 子句 | `safety: {no_internal_smb_sweep, ...}` | 已 OK |

**未来扩展建议（v0.3 草案）— 在 `2984998` 基础上增量：**

```json
{
  "schema_version": "0.3.0",
  "kind": "system_scan_recipe",
  "id": "rf_mag_oe_3d_sweep",
  "station_ref": "...",
  "devices": { "smb100a": {...}, "oe1022d": {...}, "magnetic": {...}, "laser": {...} },

  "fixed_params": {                          // 长期维持（已实现）
    "smb100a": { "rf": {...}, "fm": {...}, "lf": {...} },
    "oe1022d": { "input": {...}, "gain": {...}, "filter": {...}, ... },
    "magnetic": { "axes": {...}, "coil_matrix": {...}, ... }
  },

  "per_step_overrides": [                    // ⚠️ 新增：步级条件 patch
    {
      "match": {"step_index": [3, 4, 5]},
      "smb100a": {"rf_output_state": "ON"},
      "oe1022d": {"gain.sensitivity": "100uV"}
    }
  ],

  "sweeps": [                                // 已实现，但需增强
    {
      "sweep_id": "rf_freq_vs_bx",
      "device": "smb100a + magnetic",
      "axes": {
        "smb100a.rf.frequency_hz": {"values": [2.878e9, 2.882e9, 2.886e9]},
        "magnetic.bx_nt": {"values": [-1000.0, 0.0, 1000.0]}
      },
      "type": "cartesian_grid",
      "linked": "outer_product"              // ⚠️ 新增：显式联合语义
    }
  ],

  "acquisition_policy": { "enabled": true, ... },
  "safety": { ... }
}
```

**关键增量：**
- `per_step_overrides`：用户的"快捷覆盖一列"
- `sweeps[].linked: "outer_product | zip | expr"`：显式表达 sweep 联合方式
- 命名建议：`fixed_params` → `persistent`（更接近用户原话"长期维持"）

---

## 8. 不修改代码的承诺

本审计文档**不改动任何 crate 源码**。仅供设计讨论。
后续真正决定"缺什么 → 加什么"之前，应在 `docs/decisions/` 写明取舍。

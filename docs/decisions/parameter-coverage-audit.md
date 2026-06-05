# 设备操控参数覆盖度审计

> 本文件由用户对话整理生成，目的：对照面板截图 JSON、SCPI 设备手册、当前 Rust 命令目录
> 三方资料，识别我们与原始设计目标（"通过 JSON 配置设备参数"）之间的差距。
> **不修改任何代码。**

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

### 1.3 当前 `crates/odmr-smb100a/src/commands.rs` 实现（约 30 个函数）

已有：
- FREQuency: CW / SPAN / STARt / STOP / STEP / MODE
- POWer: LEVel / ALC
- OUTPut: STATe
- MODulation: ALL STATe
- LFOutput: STATe / FREQuency / VOLTage / SHAPe / SIMPedance
- FM: STATe / SOURce / MODE / DEViation
- SWEep: STEP / DWELl / SPACing / MODE

**对 ODMR 关键但完全缺失的：**

| 子系统 | 关键 SCPI | ODMR 实验用途 |
|--------|-----------|----------------|
| AM 调制 | `AM:STATe` `AM:SOURce` `AM:DEPTh` `AM:TYPE` | 振幅 ODMR（虽然不常用，但需可关） |
| PM 相位调制 | `PM:STATe` `PM:DEViation` `PM:SOURce` | 可与 FM 切换 |
| PULSe 脉冲调制 | `PULM:STATe` `PULM:SOURce` `PULM:TTYPe` | NV 实验常用 — Rabi 序列 |
| PGEN 脉冲发生器 | `PGENerator:STATe` `PGENerator:PERiod` `PGENerator:DOUBle:PULSe` | 控制脉冲链（与 PULM 关联） |
| PHASe 连续相位 | `PHASe` `PHASe:REFerence` | 相位连续扫描 |
| ROSCillator 参考源 | `ROSCillator:SOURce` `ROSCillator:EXTernal:FREQuency` | 时基切换 |
| LFOutput:SWEep 完整 | `LFOutput:SWEep:FREQuency:MODE` `STEP:LINear` `STEP:LOGarithmic` `DWELl` `RETRace` | LF 扫频（实验需要同步 RF 扫频） |
| LIST 模式 | `LIST:CATalog?` `LIST:DWELl` `LIST:FREQuency` `LIST:LEVel` `LIST:MODE` `LIST:TRIGger:SOURce` | 自定义离散频率/功率序列（NV Rabi 序列基础） |
| CORRection 频响修正 | `CORRection:CSET:...` `CORRection:STATe` `CORRection:VALue?` | 现场校准（重要，实验必须） |
| Power 高级 | `POWer:ATTenuation:RFOFf:MODE` `POWer:ALC:OMODe` `POWer:ALC:SONCe` | 衰减器行为、ALC 采样源（高功率安全） |
| FREQuency 高级 | `FREQuency:CENTer` `FREQuency:SPAN` `FREQuency:MULTIPlier` `FREQuency:OFFSet` `FREQuency:STEP:MODE` | 频谱仪风格扫频（中心+跨距） |
| SWEep 高级 | `SWEep:RF:MODE` `SWEep:POWer:MODE` `SWEep:EXECute` | 真正触发扫频（与 start/stop 概念不同） |

### 1.4 对照我们已有的 JSON 命令（commands.rs 约 30 个）
**SMB100A SCPI 共 ~180+ 个；当前仅 5% 覆盖。**

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

### 2.3 `crates/odmr-oe1022d/src/commands.rs` 实现

约 50+ 个函数（含 RALL? 解析），覆盖大部分面板。
**新增关键缺失：数据采样/读取子系统：**

| 命令 | 用途 |
|------|------|
| `SSLED` | 数据采样使能/配置 |
| `OUTPD` | 单点输出读取 |
| `SNAPD` | 快照式多参数读取 |
| `OAUXD` | AUX 输入读取 |
| `SPTSD` | 采样点设置 |
| `TRCAD` | trace 数据读取 |
| `INOVD` / `GNOVD` | 输入/增益过载状态 |
| `*PLLD?` | PLL 锁定状态 |

### 2.4 RALL? 采集面板

`docs/equipment_manual/oe1022d/05_oe1022d_rall_global_data_config_reading.md`
（标题暗示还有一整套"全局数据配置读取"相关 SCPI）

OE1022D 设备上还要考虑：

- `SYNCD` 同步滤波器谐波
- `HARMD` 谐波 1/2（与基频/2f/3f 实验有关）
- `ASCLD` 自动量程
- RALL? 帧内全部字段（已在 `parser.rs` 处理）

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

### 3.2 当前 `crates/odmr-maynuo-m8812/src/lib.rs`

只有 4 个 pub 项：
- `MaynuoSerialPortConfig`（配置结构）
- `MaynuoPortMetadata`（元数据）
- `MaynuoProbeError`（错误）
- `MaynuoM8812Transport`（传输层）

**没有任何 SCPI 字符串构造器！** 所有命令字符串在 `odmr-mag/src/lib.rs` 内以
散落的 `format!()` 形式存在（`format_current_command_from_ma` 等）。

**这是显著架构偏差：**
- 命令构造散落在 mag 模块中，违反 "driver 拥有全部 SCPI 封装" 的分层约束
- maynuo 驱动 crate 几乎是空的

### 3.3 缺失的命令

| SCPI | 用途 |
|------|------|
| `CURR <val>` | 设置电流（已有 format 但归在 mag） |
| `CURR?` | 读取电流 |
| `VOLT <val>` / `VOLT?` | 设置/读取电压上限 |
| `OUTP ON/OFF` | 输出开关 |
| `OUTP?` | 输出状态 |
| `MEAS:CURR?` | 实测电流 |
| `MEAS:VOLT?` | 实测电压 |
| `SYST:LOC` / `SYST:REM` | 本地/远程模式 |
| `*IDN?` | 身份查询 |
| `*RST` | 复位（危险，需禁用） |

---

## 4. 磁场（Mag / Maynuo 联动）

### 4.1 `crates/odmr-mag/src/lib.rs` 已实现

- 三轴坐标转换（笛卡尔/球坐标）
- nT ↔ mA 换算
- 各种 plan 构造（`build_safe_init_plan` 等 ~15 个）
- 零位锁定流程
- 周期 microtest

### 4.2 缺失/未充分表达

- **与 SMB 的同步 sweep**：当前 `m5b_rf_mag_oe_system_scan.recipe.json` 是
  "RF 步进 → 在每点停留 → 在每点设磁"，但 `sweeps` JSON 只支持单轴 sweep
- **多轴联动**：尚未实现 axis vector 同步 sweep（`mag_b_vector_sweep`）
- **路径 sweep**：`B = B0 + α·dir` 形式未在 schema 中表达
- **磁场 RSS 安全限**：与单轴电流限不同
- **settle_ms / ramp_ms**：recipe 只在 timing 里有默认值，没有 per-step 覆盖

---

## 5. 配方 JSON 形态的偏差

### 5.1 当前的 `examples/recipes/basic_odmr_mock.recipe.json`

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

### 5.2 用户的"快捷覆盖一列"诉求

当前 recipe 里没有"覆盖列"概念。比如：
- 用户想说："用 m5a 默认 profile，但在 step 5 把 SMB 频率从 2.87 改成 2.89"
  → 现在必须改 `sweeps` 数组
- 用户想说："OE1022D sensitivity 通常是 1mV，但这步用 100µV"
  → 现在必须创建完整新 profile

### 5.3 用户的"长期保持"诉求

有些参数是"长期维持"：
- SMB 的 ALC = AUTO（不随 step 变）
- OE1022D 的 Time Constant = 100ms（每步一致）
- Mag 的 coil matrix 标定

当前 JSON 没有把"长期保持"和"步进改变"清楚分开。

### 5.4 用户的"每步都改变"诉求

每步都变：
- SMB frequency
- Mag Bx, By, Bz
- OE1022D X 读出

schema 中已有 `sweeps`，但是多设备协调 sweep 没有显式表达：
- {"smb.frequency": 2.87e9, "mag.b_x_mt": 5.0} → {"smb.frequency": 2.88e9, "mag.b_x_mt": 5.1}
- 联合 sweep 没有 atomic 表达

---

## 6. 总结：差距清单（按优先级）

### P0 — 必须有（实验基础）

1. **SMB100A 缺失 90% SCPI**：当前只有 ~30/180+ 命令
   - **P0 子项**：`PULSe` / `PGENerator` / `LIST` / `SWEep:EXECute` / `CORRection`
2. **Maynuo 驱动 crate 几乎是空的**：命令散落在 mag
3. **OE1022D 数据采样/读取子系统**：5.2.8/5.2.9 节未实现
4. **recipe JSON 没有"per-step override"机制**

### P1 — 应该有（实验常用）

5. SMB100A `AM` `PM` `PHASe` `ROSCillator`（多调制源切换）
6. SMB100A `LFOutput:SWEep` 完整子系统
7. SMB100A `POWer:ATTenuation` / `ALC:OMODe` / `ALC:SONCe`（高功率安全）
8. 磁场多轴联合 sweep（不在单 axis 上的 vector sweep）
9. recipe 阶段分类：`static` / `persistent` / `per_step` / `co_sweep`
10. `BVectorCartesian` `BVectorSpherical` 与 SMB 频率/功率的原子联合 sweep

### P2 — 可以晚做

11. SMB100A `IQ` / `NOISe` / `DM`（暂时用不到）
12. SMB100A `CORRection:DEXChange`（频响文件导入 — 暂时手算）
13. CNI Laser 真实 driver（fake only）
14. trace service 实时降采样

---

## 7. 建议的"我们想要的样子"

针对用户原始诉求，JSON 应当支持：

```json
{
  "schema_version": "0.3.0",
  "kind": "recipe",
  "id": "rf_mag_oe_3d_sweep",
  "station_id": "station_nv_lab_01",

  "persistent": {
    "smb100a": {
      "alc_mode": "AUTO",
      "rf_output_default": "OFF",
      "modulation_global": "OFF",
      "pulse_modulation": {"state": "ON", "source": "INT", "period_s": 1e-6},
      "list_mode": {"enabled": true, "trigger_source": "AUTO"}
    },
    "oe1022d": {
      "channel_b": {
        "sensitivity": "1mV",
        "time_constant": "100ms",
        "filter_slope": "24dB",
        "harmonic": 1,
        "sync_filter": "ON"
      }
    },
    "mag": {
      "axes": ["x", "y", "z"],
      "settle_ms": 200,
      "ramp_ma_per_sec": 5.0,
      "current_max_ma": 100
    }
  },

  "per_step_overrides": [
    {
      "match": {"step_index": [3, 4, 5]},
      "smb100a": {"rf_output_state": "ON"},
      "oe1022d": {"channel_b.sensitivity": "100uV"}
    }
  ],

  "co_sweeps": [
    {
      "id": "rf_freq_vs_bx",
      "axes": {
        "smb100a.rf.frequency_hz": {"start": 2.82e9, "stop": 2.92e9, "step": 1e6},
        "mag.b_x_mt": {"start": 0.0, "stop": 10.0, "step": 0.5}
      },
      "linked": "outer_product"   // or "zip" or "expr"
    }
  ],

  "acquisition": {...},
  "safety_overrides": {...}
}
```

关键能力：
- `persistent`：每次实验长期维持的设置（不随 step 变）
- `per_step_overrides`：基于 step 索引的条件覆盖
- `co_sweeps`：多设备原子联合 sweep
- `linked: outer_product | zip | expr`：定义 sweep 的联合方式

---

## 8. 不修改代码的承诺

本审计文档**不改动任何 crate 源码**。仅供设计讨论。
后续真正决定"缺什么 → 加什么"之前，应在 `docs/decisions/` 写明取舍。

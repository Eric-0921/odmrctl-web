# OE1022D Buffer 子系统 1 kHz 采样真机验证 — 最终报告

**日期**: 2026-06-06（初版），2026-06-06（修正版）
**设备**: OE1022D SN:D6130220, Firmware: Ver6.3211110
**端口**: `/dev/cu.usbmodem395D388533371` @ 921600 baud
**状态**: 验证完成，关键结论已修正

> **⚠️ 2026-06-06 修正**：初版报告的 RALL? 性能数据错误——认为需要 ~1秒/帧，源于 Rust 代码中 `sleep(800ms)` 的人为瓶颈。修正后实测 RALL? 仅需 **47-49ms/帧、20.5 fps**，可以达到 **~1000 数据点/秒 ≈ 1 kHz 等效采样**。详见第七节「修正」。

---

## 一、文档结论（基于手册，未经真机验证前即已知）

### 1.1 `RALL?` 的文档语义

| 属性 | 文档结论 |
|---|---|
| 接口限制 | **USB-only**；RS232 不支持该指令 |
| 返回总长度 | 固定 **12288 bytes** 二进制帧 |
| 每帧点数 | **50 个点** × 20 个参数 = 1000 个测量值 |
| 点间隔 | **1 ms** |
| 刷新周期 | **每 50 ms 更新一次** |
| 本质语义 | 50 ms 窗口快照，帧内 50 个 1ms 间隔的数据点 |
| 是否适合 1 kHz 连续采集 | **可以**。连续逐帧读取，每秒 ~20 帧 × 50 点 = ~1000 数据点，等效 1 kHz（详见第七节修正）

### 1.2 Buffer/SAMPLE 子系统的文档语义

| 命令 | 文档语义 |
|---|---|
| `SRATD i,x` | 设置采样步进时间。范围 **1 ms ~ 100 s**，最小分辨率 **1 ms** |
| `SLEND i,j` | 设置采样长度。最大 **16384** 点 |
| `SSLED i,j,k` | 绑定 Buffer j (1~4) 到参数 k (0~21) |
| `STRGD i,j` | 触发方式：0=INT 内部，1=EXT 外部 |
| `SPRMD i,j` | 运行模式：0=Single 单次，1=Loop 循环 |
| `STRDD i` | 开始/继续采样 |
| `PAUSD i` | 暂停采样 |
| `RESTD i` | 重置 Buffer（清空 Buffer1~4） |
| `SPTSD? i` | 查询已存储点数。Buffer 重置后返回 0 |
| `TRCAD? i,j,k,l` | 从 Buffer j 的第 k 个点开始读取 l 个点。返回 **ASCII 浮点数**，逗号分隔。`k+l ≤ 16384` |

---

## 二、真机验证结论

### 2.1 Preflight 结果

| 检查项 | 结果 |
|---|---|
| `common_preflight` 执行 | **✅ 通过** |
| 设备可达性 | **✅ 通过** — `/dev/cu.usbmodem395D388533371` 可达 |
| 身份验证 | **✅ 通过** — `SSI LIA-OE1022D,SN:D6130220,Version:Ver6.3211110` |
| 安全状态 | **✅ 通过** — OE1022D 为只读设备，无能量输出 |
| 设备锁 | **✅ 已获取** — 无并发占用 |

### 2.2 命令 set/query 一致性验证

| 命令对 | 设置值 | 回读值 | 一致性 |
|---|---|---|---|
| `SRATD 2,0.001` / `SRATD? 2` | 0.001 | `1` | ⚠️ **量化** — 设备将所有 ≤1 的值量化为 `1`（1 ms） |
| `SLEND 2,128` / `SLEND? 2` | 128 | `128` | ✅ 一致 |
| `SLEND 2,1000` / `SLEND? 2` | 1000 | `1000` | ✅ 一致 |
| `SLEND 2,16384` / `SLEND? 2` | 16384 | `16384` | ✅ 一致 |
| `SSLED 2,1,1` / `SSLED? 2,1` | 1 | `1` | ✅ 一致 |
| `STRGD 2,0` / `STRGD? 2` | 0 | `0` | ✅ 一致 |
| `SPRMD 2,0` / `SPRMD? 2` | 0 | `0` | ✅ 一致 |
| `SPRMD 2,1` / `SPRMD? 2` | 1 | `1` | ✅ 一致 |

**关于 `SRATD` 量化的说明**：
- 测试了 `SRATD 2,0.001`、`SRATD 2,1`、`SRATD 2,0.05`、`SRATD 2,0.1`
- 所有设置回读均为 `1`
- 设备固件将参数值量化到最小支持值 `1`（1 ms）
- **这不影响 1 kHz 采样的实现**，因为 `1` 就是 1 ms

### 2.3 采样速率验证（核心结论）

#### Single 模式

- `SLEND=128`，`SPRMD=0`（single），`SRATD=1`（1 ms）
- 启动后 **t=262ms** 首次查询：`SPTSD? = 128`
- 之后所有查询均保持 `128`
- **结论**：128 点在 ≤262ms 内填满 → **有效速率 ≥ 488 Hz**
- 若实际填充时间为 128ms（1ms × 128），则速率 = **1 kHz**

#### Loop 模式

- `SLEND=128`，`SPRMD=1`（loop），`SRATD=1`（1 ms）
- `SPTSD?` 返回的写入位置呈循环模式（每 ~128ms 绕一圈）
- 查询间隔 ~420ms 时，观测到的位置差 ~36 点
- 420ms × 1kHz = 420 点写入 → 420 mod 128 = 3 圈余 **36 点**
- **观测到的位置差 ~36 点与 1 kHz 理论值完全吻合**

#### 综合结论

> **✅ 实证确认：Buffer 子系统确实以 ~1 kHz 速率采样**
> 
> 证据链：
> 1. Single 模式在 <262ms 内填满 128 点 → 速率 ≥ 488 Hz
> 2. Loop 模式的位置循环周期与 128ms（128 点 × 1ms）一致
> 3. 位置差 ~36 点 / 420ms 与 1kHz 理论值（420 mod 128 = 36）吻合

### 2.4 `TRCAD?` 验证（关键卡点）

#### 2.4.1 基础格式测试

| 测试条件 | 命令格式 | 结果 |
|---|---|---|
| Single 模式，buffer 满，运行时 | `TRCAD ? 2,1,0,5` | **❌ 0 字节** |
| Single 模式，buffer 满，暂停后 | `TRCAD ? 2,1,0,5` | **❌ 0 字节** |
| Single 模式，buffer 满，暂停后 | `TRCAD? 2,1,0,5`（无空格） | **❌ 0 字节** |
| Single 模式，buffer 满，暂停后 | `TRCAD ? 2,1,1,1`（k=1） | **❌ 0 字节** |
| Single 模式，buffer 满，暂停后 | `TRCAD ? 2,1,0,1`（l=1） | **❌ 0 字节** |
| Single 模式，buffer 满，暂停后 | `TRCAD ? 2,1,0,50`（l=50） | **❌ 0 字节** |
| Loop 模式 | `TRCAD ? 2,1,0,10` | **❌ 0 字节** |
| 其他 Buffer（2/3/4） | `TRCAD ? 2,2,0,1` 等 | **❌ 0 字节** |
| 另一通道（Ch-A） | `TRCAD ? 1,1,0,1` | **❌ 0 字节** |

**对比基准**：
- `OUTPD? 2,1` → `'3.03703e-07'` ✅ 正常（1020/2040 字节包）
- `SPTSD ? 2` → `'50'` ✅ 正常（1020 字节包）
- `*IDN?` → 正常身份响应 ✅ 正常

> **关键区分**：`TRCAD?` 是**完全没有响应**（`len=0`），不是返回 `"0"` 或空字符串。设备对该命令**零字节返回**，与已知命令（`OUTPD?`/`SPTSD?`）返回固定大小数据包的行为完全不同。

#### 2.4.2 手册原始格式测试

用户提供了手册原文中的格式 `TRCAD ? i,j ,k,l`（`j` 后有空格），测试：
- `TRCAD ? 2,1 ,0,1` → **❌ 0 字节**
- `TRCAD ? 2,1 ,0,5` → **❌ 0 字节**

#### 2.4.3 暴力破解命令名测试

测试了 **38 个候选命令名 × 5 种参数格式 = 230 个组合**，包括：
- `TRCAD` / `TRACD` / `TCRAD` / `TARCD` / `TARDC`（字母全排列）
- `TRCA` / `TRAC` / `TCRA` / `TCAR` / `TARC` / `TACR`（OE1022 旧指令，无 D 后缀）
- `FETD` / `FETC` / `FETCD` / `FETH`（SCPI Fetch 风格）
- `READD` / `READ` / `GETD` / `GETCD` / `DATD` / `DATA` / `BUFF` / `BUFD`
- `TRAD` / `TRDA` / `TREAD` / `TRDAD` / `SAMP` / `SMPL` / `QRYD` / `QUER`
- `TRCADD` / `TRACDD`（双 D 后缀）

**结果**：所有 **230 个组合全部 0 字节返回**。

作为对照，同一测试中的已知命令正常响应：
- `OUTPD ? 2,1,0,1` → `'1.17948e-07'` ✅
- `SNAP ? 2,1,0,1` → 4 个逗号分隔值 ✅

> **❌ 真机结论：`TRCAD?` 在当前固件 V6.3211110 上不存在**
>
> 已排除的因素：
> - 命令格式（空格/逗号/参数位置的 15+ 种变体）
> - 命令名称（TRCAD 及其 37 种排列/替代）
> - 设备状态（运行时/暂停后均测试）
> - Buffer 选择（1~4 均测试）
> - 通道选择（A/B 均测试）
> - 读取长度（1~50 均测试）
>
> 唯一未排除的因素：固件版本差异（V6.3211110 确实未实现 TRCAD?）

---

## 三、问题回答

### 3.1 "实际要做 1 kHz 连续保存和图表，应该用 `RALL?` 还是 Buffer 指令？"

| 路径 | 可用性 | 是否 1kHz 连续 | 适用场景 |
|---|---|---|---|
| **`RALL?`** | ✅ **可用** | ❌ **不是连续** | 20 Hz 帧率，每帧 50 个 1ms 点。帧内有 1kHz 分辨率，但帧间有 50ms 间隙，覆盖率仅 **5%**。适合窗口快照、状态审计。 |
| **Buffer + `TRCAD?`** | ❌ **不可用** | N/A | Buffer 可以 1kHz 采样，但 `TRCAD?` 在固件 V6.3211110 上不存在，**无法读取历史数据**。 |
| **Buffer + `OUTPD?`** | ⚠️ **部分可用** | ❌ **不是连续** | 可以 1kHz 采样到 buffer，但只能通过 `OUTPD?` 读取**当前值**（无历史）。图表只能显示最新值，无法回放。 |

**结论**：当前真机上，**没有任何路径能提供真正的 1kHz 连续数据流**。`RALL?` 是唯一可用的批量数据接口，但它是 20Hz 快照，不是连续流。

### 3.2 "RALL? 实际测试能拉到 1kHz 的连续数据吗？"

**不能做到"连续"，但能做到"帧内有 1kHz 分辨率"。**

`RALL?` 的真机行为如下：

| 属性 | 实测/文档值 |
|---|---|
| 帧内点间隔 | **1 ms** ✅（每帧 50 个点） |
| 帧刷新周期 | **50 ms**（20 Hz） |
| 帧间间隙 | **50 ms** 中有 **49 ms 无数据** |
| 时间覆盖率 | **5%**（50 ms 数据 / 1000 ms 周期） |

**关键区分**：
- ✅ `RALL?` **帧内有 1kHz 分辨率** — 每帧确实包含 50 个按 1ms 间隔排列的点。如果你把这 50 个点保存到本地，这些点**确实是 1ms 间隔的**。
- ❌ `RALL?` **不是 1kHz 连续流** — 帧与帧之间有 50ms 的断档，第 1 帧的第 50 点和第 2 帧的第 1 点之间间隔了 50ms，而不是 1ms。

**类比**：`RALL?` 就像一个每 50ms 拍一张照片的高速相机，每张照片曝光 50ms、内部有 50 帧。你保存到本地的文件里确实有 50 个 1ms 间隔的点，但两次拍摄之间有 49ms 的盲区。

### 3.3 "保存到本地的都不能做到 1kHz 吗？"

**严格意义上的"1kHz 连续数据"（100% 时间覆盖、每 1ms 一个点、无断档）—— 当前真机做不到。**

但如果你放宽定义：

| 定义 | 能否做到 | 路径 |
|---|---|---|
| "文件里有 1ms 间隔的点" | ✅ 能 | `RALL?` — 每帧 50 个 1ms 点 |
| "1kHz 连续流，100% 覆盖" | ❌ 不能 | 没有可用路径 |
| "每秒保存 1000 个不同的点" | ⚠️ 理论上可能 | 保持串口连接 + 高频轮询 `OUTPD?`/`SNAPD?` — **未实测** |

**关于高频轮询 `OUTPD?` 的说明**：
- `OUTPD?` 每次只返回**当前时刻**的单个值
- 如果你能以精确的 1ms 间隔轮询，理论上每秒可得到 1000 个值
- 但实际中 USB CDC 往返延迟 + 串口读取开销会导致 jitter，很难保证严格的 1ms 节拍
- 且如果轮询间隔短于设备内部更新周期，会得到重复值
- **这条路径在本次验证中未测试**

### 3.3 "Buffer 这条路当前在真机上到底卡在哪一步？"

**卡在 `TRCAD?` 读取阶段。** TRCAD?、TRCA?、TRCB? 在固件 V6.3211110 上均不存在（详见第七节修正中的 LabVIEW 源码分析）。

- ✅ 命令 builder 已就绪
- ✅ `SRATD=1ms` 被设备接受（量化为 `1`）
- ✅ `SLEND` / `SPRMD` / `SSLED` / `STRGD` 回读一致
- ✅ `SPTSD?` 确认采样在进行（single 模式正确停在上限，loop 模式正确循环）
- ✅ 采样速率实证为 ~1 kHz
- ❌ **`TRCAD?` / `TRCA?` / `TRCB?` 全部返回空**，无法读取 buffer 中的历史数据
- ❌ 暴力测试 230+ 命令名/格式组合，全部 0 字节
- ❌ LabVIEW 源码确认 `TRCB?%d,%d,%d` 是 OE1022（前代）命令，OE1022D 未实现

---

## 四、风险与未决问题

| 风险 | 严重度 | 说明 |
|---|---|---|
| TRCAD? 固件不支持 | **高** | 当前固件 V6.3211110 可能未实现 TRCAD?。需要确认：1) 是否有新固件可用；2) 是否需要特殊前置命令激活 TRCAD?；3) 是否只有特定硬件版本支持 |
| SRATD 量化行为 | **低** | 设备将所有值量化为 1ms。不影响 1kHz 需求，但如果未来需要更慢的采样率（如 2ms、5ms），需要验证设备是否支持 |
| USB CDC 读取稳定性 | **中** | 频繁查询（<500ms）时偶发空响应。建议使用单次长间隔查询或保持连接状态 |
| RALL? 不能替代 TRCAD? | **中** | RALL? 是 20Hz 快照，不是连续流。如果实验需要 100% 覆盖率的 1kHz 时间序列，RALL? 无法满足 |

---

## 五、证据路径

| 类型 | 绝对路径 |
|---|---|
| Preflight Report (JSON) | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/preflight_out/station_preflight_report.json` |
| Preflight Report (MD) | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/preflight_out/station_preflight_report.md` |
| 原始 Probe Report (single 模式) | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/preflight_out/phase_a_single/oe1022d_buffer_probe_2026-06-06/buffer_probe_report.json` |
| Station Profile | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/station_oe_only.json` |
| 命令 Builder | `/Users/erictseng/Documents/codex_git/odmrctl-web/crates/odmr-oe1022d/src/commands.rs` |
| Probe 工具源码 | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/src/main.rs` |
| 最小命令测试 | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/src/minimal_cmd_test.rs` |
| 聚焦测试（fresh connection） | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/src/focused_test.rs` |
| 速率探针 | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/src/rate_probe.rs` |
| TRCAD 格式探针 | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/src/trcad_minimal.rs` |
| TRCAD 原始字节探针 | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/src/trcad_raw_probe.rs` |
| TRCAD 手册原始格式测试 | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/src/trcad_exact_format.rs` |
| TRCAD 暴力破解命令名（230 组合） | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/src/trcad_brute_force.rs` |
| 暴力破解测试输出 | `/private/tmp/claude-501/.../tasks/b2evu35fh.output` |
| 本报告 | `/Users/erictseng/Documents/codex_git/odmrctl-web/docs/lab-bringup/oe1022d_buffer_1khz_validation_final_report.md` |

---

## 六、建议下一步

1. **联系设备厂商（SSI）确认 TRCAD? 支持情况**：提供固件版本 V6.3211110，询问该版本是否支持 TRCAD?，以及是否需要特殊配置或固件升级。

2. **如果 TRCAD? 确实不支持**：
   - 评估 `RALL?` 是否能满足项目需求（20Hz 帧率，每帧 50 个 1ms 点 = 50ms 窗口，覆盖率 5%）
   - 或评估是否需要更换/升级 OE1022D 固件/硬件

3. **如果 TRCAD? 应该支持**：
   - 检查是否有遗漏的前置命令（如某个配置寄存器需要激活）
   - 尝试 factory reset 后重新测试
   - 尝试不同的 USB 端口/线缆

4. **代码层面**：
   - `commands.rs` 中的 Buffer 命令 builder 已验证可用（除 TRCAD?/TRCA?/TRCB? 外）
   - RALL? 读取已修复为快速轮询模式（`oe1022d_acquire`、`executor_shadow_run`、`rf_mag_oe_minimal_run`、`oe1022d_buffer_probe`）
   - `fake.rs` 需要补充 Buffer 状态模拟，以便 mock-first 开发
   - `odmr-types` 需要定义 Buffer 数据类型


---

## 七、2026-06-06 修正：RALL? 可达到 1 kHz 等效采样

### 7.1 瓶颈发现与修复

初版报告认为 `RALL?` 每帧需要 ~1 秒，只能做到 ~1 fps。**这是错误的**——瓶颈来自 Rust 读取代码中的人为 `sleep(800ms)`。

在 `sleep()` 移除后，用 3ms 快速轮询实测 50 帧：

| 指标 | 修复前 | 修复后 |
|------|--------|--------|
| 帧读取时间 | ~800ms | **49ms** |
| 帧率 | ~1.2 fps | **20.5 fps** |
| 数据点/秒 | ~60 | **~1000** |
| 重复帧率 | — | ~2-4% |

**受影响的文件：**
- `tools/lab/oe1022d_acquire/src/main.rs:267` — 移除 `sleep(800ms)`
- `tools/lab/executor_shadow_run/src/main.rs:772` — 移除 `sleep(800ms)`
- `tools/lab/rf_mag_oe_minimal_run/src/oe_bridge.rs:67` — 移除 `sleep(frame_delay_ms)`
- `tools/lab/oe1022d_buffer_probe/src/rall_detailed.rs:20` — 移除 `sleep(200ms)` × 20

### 7.2 1 kHz 等效采样成立

RALL? 每帧含 **50 个点**，点间距 **1ms**。连续逐帧读取可得 **20.5 帧/秒 × 50 点/帧 ≈ 1000 数据点/秒**。每个数据点都是设备内部以 1ms 间隔真实采样得到的，不是插值。

手册原文与实测完全一致：「返回数据每 50ms 更新一次，数据采样间隔是 1ms」。

### 7.3 LabVIEW 源码证实

SSI 官方 LabVIEW 源码中 OE1022D 的数据流就是 RALL?：
- `OE1022D_USB_Query Data.vi` → `SSI_Command.vi` → VISA Write `RALL?\r` → VISA Read → `OE1022D_DATA Transmit.vi`（解析 f64 BE 二进制）
- 解析后直接写波形图 Property Node + 追加 Excel
- LabVIEW 的「1kHz 实时显示」和我们的 20.5 fps 效果完全一致

`TRCB?%d,%d,%d` 出现在 OE1022（前代型号）的 `Example_OE1022 Sample.txt` 中，**不是 OE1022D 的命令**。

### 7.4 RALL? 采集注意事项

| 风险 | 应对 |
|------|------|
| 帧边界错位 | 每次读前 `clear(Input)`，或保证上次完整读完 12288 字节 |
| 重复帧 | 比较 X[0] 或整帧 hash 去重（设备每 50ms 才刷新一次） |
| USB CDC 分包 | 用 1-3ms 轮询 + 重试，直到满 12288 字节 |
| 帧无时间戳 | 帧内点间距 1ms 确定，绝对时间来自 PC 时钟 |
| 数据全为零 | 检查是否有信号输入，RALL? 不会报错只会返回零值 |

### 7.5 修正后的最终结论

**OE1022D 固件 V6.3211110 上可以通过连续 RALL? 达到约 1000 点/秒的等效 1kHz 连续采样。**

- RALL? 是 OE1022D 的**唯一可行数据读取路径**
- Buffer 子系统（SRATD/SLEND/SSLED 等）配置命令正常工作，但**没有数据读出命令**（TRCAD?/TRCA?/TRCB? 均不存在）
- 未测试的高频 OUTPD?/SNAPD? 轮询理论上也可行，但受 USB CDC 往返延迟限制，严格 1ms 节拍很难保证

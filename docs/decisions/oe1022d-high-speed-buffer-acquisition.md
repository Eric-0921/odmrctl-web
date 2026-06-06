# OE1022D 高速采样路径决策

**Date**: 2026-06-06  
**Status**: ⛔ Superseded by `oe1022d-rall-fast-read-correction.md` — see that doc for corrected conclusions. TRCAD? does not exist on firmware V6.3211110. RALL? can achieve ~1000 data points/sec (20.5 fps × 50 points/frame) once the 800ms sleep bug is removed.

## 背景

当前仓库已经完成 `RALL?` 二进制帧的结构确认、真实抓包和解析验证，但针对“是否能支撑 1 kHz 级连续采集”这一问题，现有结论需要收敛。

已确认事实：

1. `RALL?` 返回固定 `12288 bytes` 二进制帧，其中前 `8000 bytes` 为 `20` 个参数、每个参数 `50` 个点、每点 `8 bytes` Big-Endian `f64`。
2. 手册明确写明：`RALL?` “每 `50 ms` 更新一次，返回之前 `50 ms` 的测量数据，数据采样间隔是 `1 ms`”。
3. 因此，`RALL?` 单帧内部确实包含 `1 kHz` 时间间隔的点列，但它本身是“50 ms 窗口快照”接口，不等价于可持续 `1 kHz` streaming 接口。
4. 设备手册 §5.2.8 / §5.2.9 还定义了另一套数据路径：Buffer 子系统（`SRATD` / `SLEND` / `SSLED` / `STRGD` / `SPRMD` / `STRDD` / `PAUSD` / `RESTD` / `SPTSD?` / `TRCAD?`）。
5. Buffer 子系统支持最小 `1 ms` 步进时间、最大 `16384` 点缓存深度，并通过 `TRCAD?` 以 ASCII 浮点串读取缓存内容。

这意味着：如果项目要在架构上认真支持 `1 kHz` 连续采样，不能继续把 `RALL?` 当作主路径，而必须引入 Buffer 采样路径。

## 问题

需要明确以下决策：

1. `RALL?` 与 Buffer 子系统分别承担什么角色。
2. `1 kHz` 连续采集应由哪个接口承载。
3. 仓库当前缺口位于哪些 crate。
4. executor / logging / GUI 后续应围绕哪种数据模型扩展。

## 现状审计

### `RALL?` 的定位

`RALL?` 具备以下性质：

- USB-only，RS232 不支持。
- 固定长度二进制帧，读取行为清晰，适合 parser 验证和 raw-first 记录。
- 帧中附带配置快照、overload 和 PLL lock 状态，适合审计和离线分析。
- 单帧只包含最近 `50 ms` 的窗口，设备刷新语义是 `20 Hz`。

因此，`RALL?` 适合：

- 只读快照
- parser / replay fixture
- 低速运行态质量审计
- M2/M3 阶段的受控真实链路 bring-up

`RALL?` 不适合：

- 作为 `1 kHz` 连续采样的长期热路径
- 作为前端实时波形显示的单一数据源
- 作为高覆盖率采样的唯一采集接口

### Buffer 子系统的定位

Buffer 子系统具备以下性质：

- 支持 `1 ms` 至 `100 s` 的采样步进时间，理论上可配置为 `1 kHz`。
- 每通道 4 个 Buffer，每个 Buffer 最大 `16384` 点。
- `SPTSD?` 可查询已存点数。
- `TRCAD?` 可按任意区间批量读取缓存数据。
- 数据读取为 ASCII 浮点串，不附带配置快照。

因此，Buffer 子系统更接近真正的高速采样接口，适合：

- 连续 `1 kHz` 后端采集
- 分块拉取
- 乒乓缓存
- 与 executor 状态机集成

### 当前代码缺口

`crates/odmr-oe1022d/src/commands.rs` 当前没有 Buffer 子系统命令 builder：

- 缺 `SRATD` / `SLEND` / `SSLED` / `STRGD` / `SPRMD`
- 缺 `STRDD` / `PAUSD` / `RESTD`
- 缺 `SPTSD?` / `TRCAD?`

`crates/odmr-oe1022d/src/fake.rs` 也没有 Buffer 状态模拟：

- 缺 sample time
- 缺 sample length
- 缺 buffer selector
- 缺 stored trace data

因此，当前 workspace 还不具备对 Buffer 采样路径做 mock-first 开发与验证的基础能力。

## 决策

### 决策 1：区分两条 OE1022D 数据路径

项目正式把 OE1022D 数据接口拆成两类：

1. `rall_snapshot`
2. `buffer_stream`

两者都保留，但角色不同：

- `rall_snapshot`：用于固定格式快照、状态审计、parser fixture、低速采集
- `buffer_stream`：用于 `1 kHz` 级连续采样、长时间运行、实时显示上游数据源

### 决策 2：`1 kHz` 连续采集默认走 Buffer 子系统

凡是需求表述为以下任一情形：

- “持续 1 kHz 采样”
- “高覆盖率连续时间序列”
- “前端实时波形”
- “长时间记录”

都默认基于 Buffer 子系统实现，而不是基于 `RALL?` 拉高调度频率。

### 决策 3：executor 需要显式模式切换

OE1022D executor / acquisition 状态机后续必须区分：

- `RallSnapshot`
- `BufferStream`

禁止把 Buffer 模式偷偷塞进现有 `RALL?` 流程里，以免：

- 混淆 raw 数据格式
- 混淆采样时序语义
- 让 GUI / replay / logging 无法判断数据来源

### 决策 4：热路径只做采集，不做分析

Buffer 模式仍然严格遵守 ADR-005：

- 采集线程只做 `SPTSD?` → `TRCAD?` → parse → timestamp → ring buffer → raw write
- CSV、拟合、降采样聚合、UI 图形化都在冷路径处理

## 方案比较

### 方案 A：继续强化 `RALL?`

优点：

- 现有 parser、rawbin、fixture 已成熟
- 二进制格式固定，解析简单
- 帧内带配置快照

缺点：

- 语义是 `50 ms` 窗口快照，不是连续流
- 刷新率上限与 `1 kHz` 需求存在数量级差距
- USB-only，接口灵活性差

结论：不选为 `1 kHz` 主路径。

### 方案 B：切到 Buffer 子系统

优点：

- 手册明示支持 `1 ms` 步进
- 支持批量读取、深缓存
- 支持 RS232/USB 两种链路
- 更符合 runtime 中“采集与显示分层”的架构

缺点：

- 需要新增命令 builder、ASCII parser、fake state、logging 变体
- `TRCAD?` 的 ASCII 带宽和解析开销需实测
- 需要设计单缓冲/双缓冲策略

结论：选为 `1 kHz` 主路径。

## 设计约束

### 1. 优先双 Buffer 乒乓

推荐默认策略：

- Buffer-1 采集当前窗口
- Buffer-2 供 `TRCAD?` 读取上一窗口

原因：

- 降低“边采边读”争用风险
- 降低读取时覆盖未读数据的概率
- 更容易给 executor 建立稳定节拍

单 Buffer 只可作为 bring-up 验证模式，不作为默认生产路径。

### 2. 使用水位线批量读取

推荐 executor 不按点读取，而按水位线读取，例如：

- `512`
- `1024`
- `4096`

原因：

- 减少 `TRCAD?` 调用频率
- 降低 ASCII 解析调度开销
- 降低串口/USB round-trip 对采样稳定性的影响

最终阈值以真实设备基准测试为准。

### 3. 明确 raw 数据类型

`odmr-logging` 后续应支持至少两类 OE 原始记录：

1. `rall_frame`
2. `buffer_trace_chunk`

禁止把 `TRCAD?` 的 ASCII 文本直接伪装成 `RALL?` 二进制帧写入同一格式。

### 4. GUI 只接冷路径数据

前端仍然禁止直连硬件。实时显示必须来自后端：

- ring buffer
- typed runtime API
- 降采样后的 IPC / event stream

## 性能与风险

### 性能预算

需要验证：

1. `TRCAD?` 读取 `1024` / `4096` / `8192` 点时的 wall-clock 时间
2. ASCII 解析吞吐是否足以支撑 `1 kHz`
3. 在 USB 与串口两种链路下的差异
4. `SPTSD?` 轮询频率对设备稳定性的影响

在实测前，不能假设手册上的 `1 ms` 步进自动等于端到端无丢点 `1 kHz`。

### 已知风险

1. `TRCAD?` 为 ASCII，数据体积可能显著大于二进制 `RALL?`
2. 如果读取滞后，Buffer 可能被覆盖
3. 目前 fake / harness 无法模拟 Buffer 行为
4. `common_preflight` 只覆盖身份与安全态，不覆盖 Buffer 数据路径健康度

## 影响范围

后续实现至少涉及：

- `odmr-oe1022d`：Buffer 命令 builder + `TRCAD?` 解析
- `odmr-harness`：FakeOe1022d Buffer 状态
- `odmr-types`：`BufferSample` / `TraceChunk` / `OeAcquisitionMode`
- `odmr-executor`：`buffer_stream` 状态机
- `odmr-logging`：Buffer raw chunk 持久化

GUI 不在本决策的第一阶段范围内。

## 验证计划

1. 文档阶段：确认手册、命令目录、架构边界一致
2. 真实设备阶段：严格先做 `common_preflight --preflight-only`
3. 只读验证：确认 `*IDN?` 与现有 `RALL?` 路径仍正常
4. Buffer bring-up：最小配置 `SRATD=1ms`、`SLEND<=16384`、单参数 Buffer、`STRDD` 启动、`SPTSD?` 轮询、`TRCAD?` 分块读取
5. 基准：验证是否能稳定得到 `1 kHz` 点列，而不是仅得到“窗口式快照”

## 未决问题

1. `TRCAD?` 在真实设备上的最大稳定块大小是多少。
2. USB CDC 与手册所说 RS232/USB 支持在实际机器上是否存在行为差异。
3. Buffer 读取期间，设备是否允许同时查询 overload / PLL 状态而不干扰采样。
4. `buffer_trace_chunk` 的 raw 存储格式是否应直接用 `f32`，还是保留 ASCII 原文再做离线转换。

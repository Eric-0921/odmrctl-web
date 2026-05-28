# ODMR Automation System Main PRD v0.2

> 文件定位：主 PRD / 总纲  
> 项目代号：odor-ctl-web / ODMR Automation  
> 当前阶段：架构冻结前的 v0.2 草案  
> 适用范围：NV 色心 ODMR 自动化采集系统、数据资产构建、后续机器学习数据管线  

---

## 0. 文档目的

本文件不是某一个模块的详细开发说明，而是整个 ODMR 自动化系统的主 PRD。它用于先冻结以下关键边界：

```text
1. 系统目标与非目标
2. 技术栈选择
3. Rust / Python / Tauri / AI agent 的职责边界
4. 实验 recipe 驱动方式
5. 数据采集与数据落盘原则
6. 安全与 interlock 原则
7. Sub-PRD 拆分与开发顺序
```

主 PRD 只定义全局方向和硬边界。具体 JSON schema、线程实现、设备协议、文件格式、GUI 页面、测试 harness 等细节，应下沉到各 Sub-PRD 中。

---

## 1. 项目背景

### 1.1 旧系统问题

旧 ODMR 控制项目的思路是：

```text
Python GUI + 设备驱动 + AI agent 操作层 + 手动/自动混合控制
```

这种方式可以较快完成原型，但在当前目标下存在明显风险：

```text
1. GUI、设备控制、采集、绘图、AI 调用容易混在同一进程或同一事件循环中。
2. Python 对高频串口采集、二进制解析、实时图表刷新、线程隔离不是最优选择。
3. AI agent 如果直接操作硬件，会引入不可审计、不可复现、难以安全限制的问题。
4. CSV 作为实时采集格式会导致性能、追溯性和数据一致性问题。
5. GUI 越做越重，最后会变成实验逻辑、设备状态、采集策略、数据分析的耦合中心。
```

新系统必须从一开始把实时链路、实验逻辑、GUI、AI、数据分析拆开。

---

### 1.2 实验背景

本系统服务于 NV 色心 ODMR 实验。当前典型实验链路为：

```text
激光激发金刚石 NV 色心
→ SMB100A 输出 RF 微波并进行频率调制或频率扫描
→ 外加磁场改变 NV 能级结构
→ NV 红色荧光强度随共振条件变化
→ APD 将荧光转换为电压信号
→ OE1022D 锁相放大器接收 APD 信号与参考信号
→ 通过 USB 采集锁相输出，当前重点使用 Ch-B 的 X 值
```

实验信号大致处于 uV 到 mV 区间。系统需要支持长期、多变量、可追溯采集，为后续建立数据库和机器学习模型做准备。

---

### 1.3 研究导向

本项目不能仅定位为“写一个控制软件”。更合理的研究定位是：

```text
构建面向 NV-center ODMR 的可追溯自动化采集系统，
用于生成结构化、多变量、可复现实验数据库，
并支持后续机器学习模型进行谱线拟合、磁场反演、噪声建模和实验参数优化。
```

因此系统设计的核心不是 GUI 美观，而是：

```text
1. 数据可信
2. 参数可追溯
3. 实验可复现
4. 安全边界明确
5. 自动化采集可扩展
6. 后续可支撑论文级数据集
```

---

## 2. 产品目标

### 2.1 总目标

把旧 ODMR 项目从：

```text
重 GUI + 手动配置 + agent 直接操作式自动化
```

改为：

```text
轻 GUI + JSON recipe + Rust 高性能采集核心 + Python/AI 生成实验方案 + 可追溯数据资产
```

核心变化：

```text
AI agent 不直接参与实验运行。
AI agent 只生成、检查、解释、优化 JSON recipe。
实验执行器只接受经过 schema 校验、安全校验和 dry-run 的 resolved_recipe。
```

---

### 2.2 第一阶段目标

第一阶段只实现最小可运行系统，不追求完整平台化。

必须完成：

```text
1. 设备注册与连接状态管理
2. SMB100A 基础 SCPI 控制
3. OE1022D 高性能采集核心
4. JSON recipe 基础 schema
5. recipe compiler 与 dry-run
6. 安全限制与拒绝机制
7. 实验执行器 executor
8. raw bin / index / events 实时落盘
9. 基础实时曲线显示
10. mock / replay / dry-run 测试体系
```

第一阶段成功标准：

```text
在真实设备或 fake device 环境下，系统可以加载一个基础 ODMR recipe，
通过 dry-run 展示全部危险动作与预计时长，
通过 safety check 后执行，
稳定采集 OE1022D 数据并落盘，
实验后可以 replay raw bin 并生成 parsed parquet / export csv。
```

---

### 2.3 第二阶段目标

第二阶段开始增强自动化和数据规模：

```text
1. 支持磁场 grid sweep / spherical sweep / path sweep
2. 支持 RF sweep、FM ODMR、磁场扫描组合实验
3. 支持更完整的 device_profile / block / recipe 复用
4. 支持实验队列与批量采集
5. 支持数据质量检查与自动标记
6. 支持基础谱线拟合与数据摘要
7. 支持 AI agent 生成候选 recipe，但仍必须经过静态审查
```

---

### 2.4 第三阶段目标

第三阶段面向论文与数据资产：

```text
1. 多变量 NV ODMR 数据库
2. 标准化 metadata 与 calibration snapshot
3. 可复现 benchmark dataset
4. 磁场反演模型
5. 噪声 / 漂移建模
6. 主动学习或实验参数优化
7. 可发布的数据格式与实验协议说明
```

---

## 3. 非目标

第一阶段明确不做：

```text
完整实验室平台
复杂拖拽式 GUI
AI 直接控制硬件
多 agent 在线闭环控制硬件
所有设备驱动一次性 Rust 重写
复杂插件系统
实时 CSV 保存
Web 前端直接解析 RALL? 二进制数据
GUI 线程直接读取硬件
Python 做 OE1022D 高频采集
复杂 3D 磁场仿真 Web App
自动写论文或自动给出物理结论
```

这些内容可以在未来版本评估，但不能进入 v0.2 的架构核心。

---

## 4. 系统设计原则

### 4.1 JSON recipe 驱动

所有实验运行必须来自结构化 recipe：

```text
station
profile
block
recipe
resolved_recipe
run_event
```

原则：

```text
1. 人类通过 GUI 或编辑器选择 / 修改 recipe。
2. AI agent 可以生成 recipe 草案。
3. compiler 负责展开 sweep、block、profile 引用。
4. executor 只接受 resolved_recipe。
5. recipe 不能覆盖 safety limit。
6. 未通过 schema / safety / dry-run 的 recipe 不能执行。
```

---

### 4.2 AI 不直接控制硬件

AI agent 的边界：

```text
允许：
  生成 recipe 草案
  解释 recipe
  检查参数合理性
  生成实验计划
  生成数据分析脚本
  总结实验结果

禁止：
  直接访问硬件
  直接发送 SCPI / serial 命令
  修改 safety limit
  绕过 dry-run
  绕过 schema validation
  在实验运行中直接插入动作
```

AI 的输出必须进入普通工程路径：

```text
AI output
→ recipe draft
→ schema validation
→ compiler
→ safety check
→ dry-run
→ user approval
→ executor
```

---

### 4.3 GUI 不做实验逻辑

GUI 只负责展示和交互：

```text
设备连接
recipe 选择
dry-run 展示
run / stop 控制
实时曲线
状态监控
magnetic planner 入口
```

GUI 不负责：

```text
实验逻辑
硬件采集
RALL? 解析
CSV 转换
AI 生成
复杂数据分析
直接访问仪器
```

---

### 4.4 实时路径最小化

实时采集链路必须尽可能短：

```text
OE1022D transport thread
→ raw frame queue
→ parser thread
→ ring buffer
→ raw bin writer
→ downsampled UI stream
```

禁止：

```text
采集线程画图
采集线程做复杂分析
采集线程写 CSV
GUI 线程读取串口
普通状态 query 打断 RALL? 高频采集
```

---

### 4.5 Raw-first 数据原则

实时阶段只写：

```text
raw bin
index.jsonl
events.jsonl
```

实验后再生成：

```text
parsed.parquet
export.csv
summary.json
figures
```

CSV 只是导出格式，不是实时采集格式，也不是主数据格式。

---

## 5. 技术栈决策

### 5.1 推荐技术路线

```text
Frontend GUI:       Tauri + Web 前端
Frontend language:  TypeScript
Chart:              uPlot 优先，ECharts 作为复杂仪表盘备选
Backend core:       Rust
OE1022D acquisition: Rust
SMB100A control:    Rust SCPI over LAN socket 起步；Python RsInstrument 作为 fallback / validation
Recipe / AI:        Python
Data analysis:      Python + Polars / DuckDB / Jupyter
Storage:            raw bin + JSONL + Parquet + CSV export
```

---

### 5.2 Rust 负责什么

Rust 负责所有对实时性、可靠性、安全边界要求高的部分：

```text
设备连接状态机
OE1022D 高频采集
RALL? raw frame 读取
binary parser
ring buffer
raw bin recorder
SMB100A SCPI socket 控制
magnetic safety check
recipe executor
resource lease
event logging
UI 查询 API
mock / replay / benchmark
```

Rust 不负责：

```text
prompt / AI 生成逻辑
复杂 notebook 分析
模型训练主流程
论文图后处理
```

---

### 5.3 Python 负责什么

Python 负责科研语义、AI、离线分析：

```text
AI recipe generation
recipe lint / explain
离线数据分析
谱线拟合
机器学习模型训练
notebook 工作流
数据集统计
```

Python 不负责：

```text
OE1022D 高频采集
实时图表刷新
核心 executor
安全 interlock 执行
硬件实时控制闭环
```

---

### 5.4 Tauri / Web GUI 负责什么

Tauri 提供桌面壳和 Rust 后端调用边界。Web 前端负责交互。

GUI 要求：

```text
轻量
状态清晰
不持有硬件资源
不解析 raw binary
不阻塞采集线程
```

实时图表要求：

```text
刷新率：10–30 Hz
显示点数：1000–5000 点
后端降采样
前端只接收可绘制 trace
```

---

### 5.5 Go 的位置

Go 暂不作为主技术栈。

理由：

```text
1. Tauri 后端天然是 Rust。
2. 本项目核心痛点是采集、解析、线程隔离、二进制数据、实时落盘，不是大规模 HTTP 服务。
3. 引入 Go 会形成 Rust + Go + Python + TypeScript 的过早多语言复杂度。
```

Go 可作为未来远程实验服务、服务器端调度或数据服务的候选，但不进入 v0.2 主架构。

---

## 6. 设备范围

### 6.1 第一阶段设备

```text
SMB100A RF / microwave signal generator
OE1022D DSP lock-in amplifier
Laser controller
Mag X / Y / Z axis current source or coil controller
APD signal path
```

---

### 6.2 SMB100A 定位

SMB100A 是实验参数源，不是高吞吐数据源。

第一阶段支持：

```text
*IDN?
RF frequency
RF power
RF output on/off
frequency mode CW / sweep
LF generator frequency / shape / voltage
FM source / deviation / state
modulation global on/off
safe preset / safe shutdown
```

实现策略：

```text
优先：Rust TCP socket SCPI
保留：Python RsInstrument / PyVISA fallback
暂缓：Rust VISA FFI
```

---

### 6.3 OE1022D 定位

OE1022D 是数据主链路核心。

第一阶段重点：

```text
Ch-B X value
Ch-B Y / R / theta 可扩展
reference status
sensitivity
time constant
filter slope
overload state
RALL? raw frame acquisition
```

必须用 Rust 实现高频采集核心。

---

### 6.4 Laser 定位

Laser 在第一阶段只作为受控状态设备：

```text
connect / disconnect
power setpoint
emission on/off
warmup / settle time
max power safety limit
safe shutdown
```

第一阶段不做复杂激光噪声建模或闭环功率控制。

---

### 6.5 Magnetic axes 定位

磁场模块是复杂模块，应单独 Sub-PRD 处理。

必须支持：

```text
B vector 表达
笛卡尔坐标
球坐标
coil matrix
zero offset
current limit
ramp limit
settle time
grid sweep
spherical sweep
path sweep
```

第一阶段暂不做：

```text
复杂 3D 磁场仿真
大型磁场可视化系统
磁场反演模型在线闭环
```

---

## 7. 核心数据模型

### 7.1 顶层对象

```text
station
  描述当前实验站、设备列表、连接方式、安全限制、默认目录。

device_profile
  描述某台设备的静态配置、常用配置、能力范围、默认安全参数。

block
  可复用实验片段，例如 RF sweep block、magnetic grid block、laser setup block。

recipe
  用户或 AI 编写的实验方案，允许引用 profile 和 block。

resolved_recipe
  compiler 展开后的最终执行步骤。executor 只接受 resolved_recipe。

run_event
  实验运行中产生的事件、命令、状态、错误、stop、warning。
```

---

### 7.2 数据点最低追溯字段

每个有效数据点必须能追溯到：

```text
run_id
step_id
substep_id
timestamp_monotonic_ns
timestamp_wall
recipe_hash
resolved_recipe_hash
station_snapshot_hash
device_state_hash
```

建议采样表字段至少包括：

```text
B_x
B_y
B_z
B_abs
B_theta
B_phi
coil_current_x
coil_current_y
coil_current_z
smb_freq_hz
smb_power_dbm
smb_output_state
smb_mod_state
smb_fm_dev_hz
smb_lf_freq_hz
laser_power_mw
laser_state
oe_ch_b_x
oe_ch_b_y
oe_ch_b_r
oe_ch_b_theta
oe_ref_freq_hz
oe_sensitivity
oe_time_constant
oe_filter_slope
oe_overload_state
```

---

## 8. Runtime Model

### 8.1 进程边界

第一阶段推荐单桌面应用，多内部任务：

```text
Tauri App Process
  ├─ WebView Frontend
  ├─ Rust Command API
  ├─ Device Manager
  ├─ Recipe Compiler / Executor interface
  ├─ OE1022D acquisition task
  ├─ SMB100A control task
  ├─ Data recorder task
  └─ UI data service

Python Process / Script
  ├─ AI recipe generation
  ├─ offline analysis
  └─ notebook / model training
```

Python 不常驻控制硬件。Python 输出文件或通过受限 API 提交 recipe 草案。

---

### 8.2 线程 / task 边界

建议模型：

```text
main ui command task
  只处理 UI 命令、状态查询、run 控制。

device manager task
  管理连接、断开、resource lease、设备状态缓存。

oe acquisition thread
  只做 RALL? 或高速采集读取，不做 GUI、不做复杂分析。

oe parser thread
  解析 raw frame，写入 ring buffer 和 recorder queue。

recorder thread
  写 raw bin / index / events。

executor task
  按 resolved_recipe 执行步骤，发命令，记录事件。

ui trace service
  从 ring buffer 读取窗口数据，降采样后返回前端。
```

---

### 8.3 资源租约

所有设备运行前必须申请资源租约：

```text
resource lease:
  smb100a
  oe1022d
  laser
  mag_x
  mag_y
  mag_z
```

原则：

```text
1. 同一时间只能有一个 executor 持有关键设备资源。
2. GUI 状态查询不能抢占采集资源。
3. emergency stop 可以打断全部 lease。
4. manual override 必须被事件日志记录。
```

---

## 9. Safety Model

### 9.1 安全原则

```text
AI 不能修改 safety limit。
recipe 不能覆盖 safety limit。
安全失败时不能执行。
所有危险命令必须 dry-run 可见。
实际执行值必须来自 resolved_recipe 和 station safety policy 的交集。
```

---

### 9.2 必须定义的安全限制

```text
laser max power
laser warmup / settle time
SMB max power
SMB allowed frequency range
SMB output default off
mag max current
mag max ramp rate
mag settle time
OE1022D overload handling
emergency stop
safe shutdown
manual override rules
resource lease rules
```

---

### 9.3 拒绝执行条件

以下情况必须拒绝执行：

```text
schema validation failed
unresolved recipe
safety_report failed
device not connected
required device lease denied
SMB power exceeds limit
mag current exceeds limit
mag ramp exceeds limit
laser power exceeds limit
unknown command
unknown device profile
dry-run not approved
station snapshot missing
```

---

## 10. Recipe Workflow

### 10.1 编译流程

```text
recipe.json
→ schema validation
→ resolve profile references
→ expand blocks
→ expand sweeps
→ insert dwell / settle / average
→ estimate duration
→ generate resolved_recipe.json
→ generate safety_report.json
→ dry-run preview
→ user approval
→ executor run
```

---

### 10.2 compiler 职责

compiler 负责：

```text
引用 profile
展开 sweep
展开 block
计算预计时长
插入 dwell / settle / average
生成 resolved_recipe.json
生成 safety_report.json
```

compiler 不碰硬件。

---

### 10.3 executor 职责

executor 负责：

```text
申请设备资源租约
按步骤执行
发送设备命令
等待 dwell / settle
控制采样窗口
记录 events.jsonl
处理 stop / error / emergency
触发 safe shutdown
```

executor 不接受：

```text
未 resolved 的 recipe
schema 未通过的 recipe
safety_report failed 的 recipe
未 dry-run 审核的 recipe
```

---

## 11. Timing / Synchronization / Averaging 原则

### 11.1 必须记录的时间

```text
monotonic timestamp
wall timestamp
设备命令发送时间
设备命令返回时间
SMB dwell time
磁场 settle time
laser warmup / settle
OE1022D time constant
采样窗口开始 / 结束
前段数据丢弃时间
重复测量编号
```

---

### 11.2 软件同步模型

第一阶段采用软件同步，不承诺硬实时。

原则：

```text
1. 所有时间戳以 monotonic clock 为主。
2. wall clock 只用于人类可读记录。
3. 命令发送、返回、settle、采样窗口必须记录事件。
4. 每个采样窗口必须知道自己属于哪个 recipe step。
5. OE1022D time constant 影响有效采样窗口，不能忽略。
```

---

### 11.3 Averaging 原则

每个 step 可定义：

```text
settle_before_sample
sample_window
reject_initial_window
repeat_count
average_method
quality_flags
```

第一阶段至少支持：

```text
mean
median
std
peak_to_peak
raw keep
```

---

## 12. Data Logging Model

### 12.1 Run directory

每次实验必须创建独立 run directory：

```text
runs/
  2026-xx-xx_run_id/
    raw/
      oe1022d.rawbin
      index.jsonl
    metadata/
      station_snapshot.json
      resolved_recipe.json
      safety_report.json
      device_idn.json
      calibration_snapshot.json
    events/
      events.jsonl
      warnings.jsonl
      errors.jsonl
    parsed/
      samples.parquet
      sweep_points.parquet
      fitted_resonances.parquet
    exports/
      export.csv
      summary.json
```

---

### 12.2 实时写入原则

实时阶段只写：

```text
raw bin
index.jsonl
events.jsonl
```

禁止实时写：

```text
CSV
Excel
复杂 parquet conversion
图像文件
模型结果
```

---

### 12.3 追溯原则

每个数据点必须能回溯到：

```text
recipe step
设备状态
station snapshot
安全报告
运行事件
原始 raw frame
parser version
software version
```

---

## 13. GUI Model

### 13.1 GUI 页面范围

第一阶段 GUI 至少包括：

```text
Dashboard
  当前设备连接状态、运行状态、warning、emergency stop。

Device Registry
  连接 / 断开 / IDN / 设备状态缓存。

Recipe Panel
  选择 recipe、显示 schema check、compile、dry-run。

Run Control
  start / pause / stop / emergency stop。

Live Chart
  Ch-B X / Y / R / theta 实时曲线，后端降采样。

Run Browser
  查看历史 run directory、metadata、events、export。

Magnetic Planner
  生成 magnetic block JSON，预览 coil current，检查 safety。
```

---

### 13.2 GUI 性能要求

```text
图表刷新：10–30 Hz
显示点数：1000–5000
前端不接触 raw frame
前端不解析 binary
后端返回降采样结果
UI 卡顿不得影响采集
```

---

## 14. Test Harness

### 14.1 必须支持的 fake device

```text
fake OE1022D
fake RALL frame generator
fake SMB100A SCPI server
fake laser serial
fake mag axis
```

---

### 14.2 必须支持的测试

```text
recipe validation tests
dry-run tests
safety rejection tests
raw bin replay tests
timestamp alignment tests
parser unit tests
executor integration tests
GUI non-blocking tests
long-run acquisition tests
```

---

### 14.3 无设备可运行原则

代码必须在没有真实设备连接时运行：

```text
cargo test
mock run
dry-run
raw replay
fake scpi server integration
```

有设备连接时，必须支持真实串口 / socket：

```text
发送实际指令
接收实际设备响应
记录真实 IDN
记录真实事件
```

---

## 15. Agent Workflow

### 15.1 每个开发 agent 必须定义

```text
scope
inputs
outputs
forbidden areas
test requirements
review checklist
```

---

### 15.2 Agent 示例：OE1022D Core Agent

```text
Scope:
  实现 OE1022D Rust acquisition core。

Inputs:
  03 OE1022D Acquisition Sub-PRD
  fake RALL frame spec
  OE1022D command manual

Outputs:
  Rust parser
  ring buffer
  recorder
  replay tool
  benchmark report
  tests

Forbidden:
  不改 GUI
  不改 recipe schema
  不碰 SMB100A
  不改 safety policy

Tests:
  parser unit tests
  raw replay tests
  30 min acquisition stability test
```

---

## 16. Sub-PRD 列表

### 16.1 文件结构

```text
docs/
  prd/
    00_main_prd.md
    01_architecture_prd.md
    02_device_registry_prd.md
    03_oe1022d_acquisition_prd.md
    04_recipe_schema_prd.md
    05_recipe_compiler_executor_prd.md
    06_timing_sync_averaging_prd.md
    07_data_logging_prd.md
    08_gui_tauri_chart_prd.md
    09_magnetic_planner_prd.md
    10_safety_interlock_prd.md
    11_harness_mock_replay_prd.md
    12_agent_workflow_prd.md

  adr/
    ADR-001-tauri-ui.md
    ADR-002-rust-oe1022d-core.md
    ADR-003-smb100a-scpi-socket-first.md
    ADR-004-json-recipe-driven.md
    ADR-005-no-ai-live-hardware.md
    ADR-006-raw-bin-before-csv.md

  schemas/
    station.schema.json
    profile.schema.json
    block.schema.json
    recipe.schema.json
    resolved_recipe.schema.json
    event.schema.json

  examples/
    odmr_basic_sweep.recipe.json
    odmr_bz_sweep.recipe.json
    odmr_lf_fm_sweep.recipe.json
    mag_vector_grid.block.json
```

---

### 16.2 Sub-PRD 职责

| 编号 | 文件 | 职责 |
|---:|---|---|
| 00 | main_prd | 总目标、边界、技术栈、路线图 |
| 01 | architecture_prd | 模块依赖、线程模型、进程模型、禁止循环依赖 |
| 02 | device_registry_prd | device_id、transport、port、idn、capabilities、connection status |
| 03 | oe1022d_acquisition_prd | RALL?、raw frame、parser、ring buffer、raw replay、benchmark |
| 04 | recipe_schema_prd | station/profile/block/recipe/resolved_recipe schema |
| 05 | compiler_executor_prd | compile、dry-run、resource lease、executor、events |
| 06 | timing_sync_averaging_prd | timestamp、settle、dwell、采样窗口、平均策略 |
| 07 | data_logging_prd | run directory、raw bin、index、events、parquet、csv export |
| 08 | gui_tauri_chart_prd | GUI 页面、图表、API、前后端边界 |
| 09 | magnetic_planner_prd | B vector、coil matrix、sweep、safety preview |
| 10 | safety_interlock_prd | laser/RF/mag/OE safety、emergency stop、manual override |
| 11 | harness_mock_replay_prd | fake devices、mock run、dry-run tests、replay tests |
| 12 | agent_workflow_prd | AI / coding agent 的 scope、输入输出、禁止区、review checklist |

---

### 16.3 PRD 编写顺序

第一轮：冻结边界

```text
00_main_prd.md
01_architecture_prd.md
10_safety_interlock_prd.md
11_harness_mock_replay_prd.md
```

第二轮：完成最小运行核心

```text
03_oe1022d_acquisition_prd.md
07_data_logging_prd.md
04_recipe_schema_prd.md
05_recipe_compiler_executor_prd.md
```

第三轮：完善上层能力

```text
08_gui_tauri_chart_prd.md
09_magnetic_planner_prd.md
12_agent_workflow_prd.md
02_device_registry_prd.md
06_timing_sync_averaging_prd.md
```

---

## 17. Milestones

### 17.1 M0：PRD v0.2 完整骨架

交付：

```text
00–12 PRD 均有完整骨架
每个 PRD 都有 scope / non-goals / inputs / outputs / safety / acceptance
ADR 初稿完成
```

验收：

```text
新开发者只看 PRD，可以理解系统边界。
AI agent 可以根据某个 Sub-PRD 独立完成受限任务。
```

---

### 17.2 M1：Mock-first 最小系统

交付：

```text
fake OE1022D
fake SMB100A SCPI server
recipe schema validation
compiler dry-run
executor mock run
events.jsonl
raw bin replay demo
```

验收：

```text
无真实设备时能完整跑通 basic_odmr recipe。
所有危险动作在 dry-run 中可见。
安全失败会拒绝执行。
```

---

### 17.3 M2：真实 OE1022D 采集核心

交付：

```text
oe1022d-core crate
RALL? acquisition
parser tests
ring buffer
raw bin recorder
replay tool
benchmark report
```

验收：

```text
真实设备连续采集 30 min。
UI 卡顿不影响采集。
raw bin 可回放。
解析结果与旧 Python 结果一致或差异可解释。
```

---

### 17.4 M3：真实 SMB100A + 基础 ODMR run

交付：

```text
SMB100A SCPI socket driver
RF / LF / FM / sweep 基础控制
safe preset
basic ODMR recipe
real run directory
parsed parquet
export csv
```

验收：

```text
能够自动执行基础 RF sweep 或 FM ODMR 采集。
每个数据点可回溯到 recipe step 与设备状态。
```

---

### 17.5 M4：GUI v0.1

交付：

```text
Tauri desktop app
device panel
recipe panel
dry-run panel
run control
live chart
run browser
```

验收：

```text
GUI 不直接访问硬件。
前端不解析 raw frame。
图表 10–30 Hz 刷新。
停止与 emergency stop 可用。
```

---

### 17.6 M5：数据集准备阶段

交付：

```text
batch recipe
magnetic sweep
quality flags
summary statistics
dataset manifest
baseline fitting notebook
```

验收：

```text
可以连续生成多 run 数据集。
数据可复现、可追溯、可导出、可用于 ML baseline。
```

---

## 18. Acceptance Criteria

### 18.1 系统级验收

```text
1. 系统可以在无设备模式下完整 dry-run。
2. 系统可以在 fake device 下完整 mock-run。
3. 系统可以在真实 OE1022D 下稳定采集。
4. 系统可以在真实 SMB100A 下执行基础 RF / FM 设置。
5. 安全失败必须拒绝执行。
6. AI 不能绕过 schema / safety / dry-run。
7. GUI 卡顿不能影响采集。
8. 实时阶段不写 CSV。
9. 每个数据点能回溯到 recipe step 和设备状态。
10. raw bin 能 replay 并生成 parsed parquet。
```

---

### 18.2 论文数据导向验收

```text
1. 每个 run 有完整 station_snapshot。
2. 每个 run 有 resolved_recipe。
3. 每个 run 有 safety_report。
4. 每个 run 有 events.jsonl。
5. 每个 run 有 raw bin。
6. 每个 parsed 数据点有 step_id 与 timestamp。
7. 每个数据导出都能追溯到原始 raw bin。
8. 数据集可以生成 manifest。
9. 数据集可以被 notebook 复现读取。
10. 数据质量 flag 可机器读取。
```

---

## 19. Open Questions

以下问题留给后续 Sub-PRD 或实验验证：

```text
1. OE1022D RALL? 实际稳定帧率是多少？
2. RALL? raw frame 的完整二进制格式是否已完全确认？
3. SMB100A 优先使用 LAN socket 还是现有 VISA 连接？
4. 当前实验室磁场线圈的 coil matrix 如何标定？
5. APD 电压信号是否需要额外采集路径或 calibration？
6. laser controller 的真实 serial protocol 是否完整？
7. 第一版数据质量 flag 应包括哪些？
8. ODMR 谱线拟合 baseline 使用 Lorentzian、multi-Lorentzian 还是 derivative model？
9. 需要如何定义 ML dataset 的 train / validation / test split？
10. 是否需要把温度、环境光、实验人员操作事件也纳入 metadata？
```

---

## 20. 决策摘要

当前 v0.2 主决策：

```text
语言：
  Rust: 实时采集、设备执行器、SMB100A SCPI socket、数据落盘、磁场安全
  TypeScript: Web GUI
  Python: AI recipe 生成、离线分析、模型训练、notebook

GUI：
  Tauri + Web frontend + uPlot

SMB100A：
  v0.2 优先 Rust TCP socket SCPI
  Python RsInstrument / PyVISA 保留为 fallback 和 validation
  VISA FFI 暂缓

OE1022D：
  Rust serial / USB acquisition core
  raw bin + replay test mandatory

AI：
  只生成 JSON / 分析 / 解释
  不直接碰硬件
  不覆盖 safety limit

数据：
  实时 raw bin / index / events
  实验后 parquet
  CSV 仅导出

论文方向：
  自动化数据集 + benchmark + ML / 磁场反演 / 噪声建模
```

---

## 21. 附录：术语

```text
ODMR
  Optically Detected Magnetic Resonance，光探测磁共振。

NV center
  金刚石氮-空位色心。

recipe
  用户或 AI 编写的实验方案。

resolved_recipe
  compiler 展开后的最终执行方案。

station
  实验站配置，包括设备、连接、安全限制、目录等。

profile
  设备静态配置和默认参数集合。

block
  可复用实验片段。

run
  一次实验运行。

raw bin
  实时保存的原始二进制数据。

index.jsonl
  raw bin 的索引文件。

events.jsonl
  实验事件日志。

resource lease
  执行器对设备资源的独占租约。

safety_report
  recipe 编译后生成的安全检查报告。

dry-run
  不触碰真实危险输出的预执行展示。
```

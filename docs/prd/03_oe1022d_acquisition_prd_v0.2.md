# Sub-PRD 03: OE1022D Acquisition PRD v0.2

> 文件定位：OE1022D 高性能采集核心 Sub-PRD  
> 所属项目：odor-ctl-web / ODMR Automation  
> 上级文档：00_main_prd.md v0.2、01_architecture_prd.md v0.2、02_device_registry_connection_prd.md v0.2  
> 当前阶段：架构冻结前的 v0.2 草案  
> 目标读者：Rust 后端开发者、采集核心开发者、Executor 开发者、Data Logging 开发者、GUI 开发者、AI/coding agent  

---

## 0. 文档目的

本文件定义 ODMR 自动化系统中的 **OE1022D Acquisition Core** 模块。

该模块负责通过 OE1022D DSP Lock-in Amplifier 获取锁相数据，并把采集、解析、缓冲、落盘、回放、降采样查询与错误恢复建立为独立的 Rust 核心模块。

本模块是新系统中最重要的实时路径之一。它必须避免旧系统中 GUI、串口、状态查询、绘图、CSV 保存、实验逻辑互相抢占的结构性问题。

```text
OE1022D Acquisition Core 负责：
  RALL? / 数据读取命令的稳定采集循环
  raw frame 接收
  raw frame timestamp
  parser 解析
  parsed sample 生成
  ring buffer 缓冲
  raw bin 落盘
  index.jsonl 记录
  UI 降采样查询接口
  replay 工具
  mock frame 和 parser tests
  benchmark report
```

```text
OE1022D Acquisition Core 不负责：
  GUI 线程绘图
  Web 前端解析二进制 frame
  实时 CSV 写入
  SMB100A 控制
  激光器控制
  磁场扫描路径规划
  recipe 展开
  AI 生成实验方案
  论文级拟合和机器学习训练
```

---

## 1. Purpose

本模块解决的问题是：**把 OE1022D 锁相放大器的数据采集从 GUI 和实验流程中剥离出来，形成可测试、可回放、可 benchmark、可长期稳定运行的 Rust 采集核心。**

当前实验目标是 NV-center ODMR 自动化采集。实验链路中：

```text
NV 红色荧光
→ APD 电平信号
→ OE1022D Signal IN
→ OE1022D 锁相解调
→ USB / serial 读取数据
→ Rust acquisition core
→ raw bin + ring buffer
→ parsed parquet / UI downsample
```

OE1022D 在本系统中通常关注 Ch-B。Ch-B 的核心读数包括：

```text
X
Y
R
theta
Freq
Noise
harmonic channels if enabled
```

其中：

```text
X      同相分量，适合 ODMR 斜率处差分 / 导数型信号
Y      正交分量，用于判断相位是否调准
R      幅值，适合普通幅值型检测
θ      相位
Freq   参考频率识别结果
Noise  噪声读数
```

第一阶段默认采集目标为：

```text
primary_channel = Ch-B
primary_value   = B-X or B-R, selected by recipe/profile
reference       = external reference from SMB100A LF / modulation reference
```

但采集核心不能把 Ch-B 写死。系统必须保留 Ch-A / Ch-B / both 的解析能力，避免后续扩展困难。

---

## 2. Background and Constraints

### 2.1 实验上下文

当前 ODMR 系统使用 SMB100A 提供微波和低频调制参考，OE1022D 接收 APD 信号和参考信号。常见配置为：

```text
APD signal → OE1022D Ch-B Signal IN
SMB100A LF / reference → OE1022D Ch-B REF IN
OE1022D Ch-B lock-in output → software acquisition
```

采集核心必须记录信号来源、通道、参考状态和解析字段，否则后续数据无法用于数据库和机器学习。

### 2.2 OE1022D 配置注意点

OE1022D 的 A/B 通道需要分别配置。不能假设 Ch-A 的配置会影响 Ch-B。Ch-B 的输入、滤波、灵敏度、参考、相位和输出配置都必须可追溯。

推荐的 Ch-B 初始实验配置由 profile 管理，而不是采集核心硬编码：

```text
input_source: voltage_single_ended
coupling: AC
shield_grounding: Ground or Float, according to noise situation
notch_filter: None or Both if power-line interference exists
reference_source: External
external_ref_trigger: TTL Rising Edge or Sine Zero Crossing
phase: 0 initially, then Auto Phase if stable
dynamic_reserve: Normal, fallback High Reserve if overload
sensitivity: start high range, then auto/manual refine
time_constant: 100 ms - 300 ms initial
filter_slope: 12 dB/oct initial
sync_filter: Off for 500 Hz reference in first stage
primary_output: B-X or B-R
```

采集核心只负责读取和记录，不负责决定这些实验参数是否科学。参数决策属于 recipe/profile/safety/timing 模块。

### 2.3 旧系统瓶颈

旧 Python/PyQt 系统中容易出现：

```text
GUI 状态轮询
RALL? 采集
设备命令查询
图表刷新
CSV 保存
AI / agent 操作
```

共享同一个事件循环、串口 session 或 Python GIL 资源，导致采集抖动、UI 卡顿和数据落盘不可靠。

新系统必须将实时路径压缩为：

```text
Serial acquisition thread
→ raw frame queue / ring buffer
→ parser thread
→ parsed sample ring buffer
→ raw bin writer
→ UI downsample API
```

---

## 3. Scope

本 PRD 负责定义：

```text
1. oe1022d-core Rust crate 的职责
2. RALL? / acquisition command 调度模型
3. 采集周期和采集模式
4. raw frame size 与 frame boundary 策略
5. serial transport 读写策略
6. 采集线程、parser 线程、writer 线程边界
7. monotonic timestamp 与 wall timestamp 记录
8. ring buffer 数据结构
9. raw bin 文件格式
10. index.jsonl 写入规则
11. parsed sample 数据模型
12. UI 查询接口
13. downsample 策略
14. 状态查询对采集循环的影响限制
15. 错误恢复策略
16. drop frame 策略
17. mock RALL frame
18. raw replay tool
19. parser tests
20. benchmark report
21. agent 开发边界
```

---

## 4. Non-goals

本模块第一阶段明确不负责：

```text
1. OE1022D 全部面板设置 GUI 复刻
2. OE1022D 所有远程命令完整封装
3. SMB100A 扫频控制
4. 激光器控制
5. 磁场控制
6. recipe compiler
7. 实验 executor 主状态机
8. 数据拟合
9. 机器学习训练
10. notebook 分析
11. 实时 CSV 输出
12. Web 前端解析 raw frame
13. Web 前端直接连接串口
14. 采集线程绘图
15. 采集线程执行复杂统计分析
16. Python 高频采集
17. agent 直接控制 OE1022D
```

特别说明：

```text
本模块可以提供 read_latest_snapshot() 给 GUI 查询；
但 GUI 不能调用 serial.write("RALL?")。

本模块可以提供 get_downsampled_window() 给图表；
但前端不能接触 raw binary frame。

本模块可以把 raw frame 写入 raw bin；
但不能在实时路径中同步写 CSV。
```

---

## 5. Design Principles

### 5.1 采集优先级高于显示

采集稳定性优先于 GUI 刷新率。

```text
正确优先级：
  raw acquisition > raw bin writing > parsed sample > downsampled UI > human visual smoothness
```

如果 UI 卡顿，采集不能暂停。  
如果 parser 暂时落后，raw frame 仍必须尽量落盘。  
如果 writer 压力过大，系统必须记录 backpressure event，而不是悄悄丢数据。

---

### 5.2 raw-first

实时路径中优先保存原始数据。

```text
实时写：
  raw/oe1022d.rawbin
  raw/oe1022d.index.jsonl
  events/events.jsonl

实验后生成：
  parsed/oe1022d_samples.parquet
  exports/oe1022d_export.csv
```

CSV 只作为导出格式，不作为实时采集格式。

---

### 5.3 timestamp-first

每一帧 raw frame 和每一个 parsed sample 都必须有时间戳。

至少包含：

```text
t_mono_recv_ns      读到完整 frame 的 monotonic timestamp
t_wall_recv_ms      对应 wall clock timestamp
t_mono_query_ns     如果是 query-response 模式，记录发送命令时间
t_mono_read_start_ns
read_duration_ns
parse_duration_ns
sequence_no
```

后续 timing/sync/averaging 模块会使用这些字段计算 dwell、settle、前段丢弃、平均窗口等。

---

### 5.4 采集线程不做复杂工作

采集线程只做三件事：

```text
1. 按规定节奏向 OE1022D 请求或读取数据
2. 读取 raw bytes / raw text frame
3. 为 raw frame 打时间戳并推入 queue
```

采集线程禁止：

```text
1. 画图
2. 写 CSV
3. 做拟合
4. 做机器学习
5. 展开 recipe
6. 判断 ODMR 峰位
7. 频繁查询设备状态
8. 执行 SMB100A / laser / mag 命令
```

---

### 5.5 parser 可失败但不能污染 raw

raw frame 是事实记录。parser 失败只说明当前 parser 无法理解该 frame，不代表 raw 数据无效。

parser 失败时必须：

```text
1. 保留 raw frame
2. 写 parse_error event
3. 标记 frame index parse_status = failed
4. 不生成伪造 sample
5. 允许 replay 后用新版 parser 重新解析
```

---

### 5.6 mock 与真实采集同接口

开发环境必须能在没有设备连接时运行。

```text
real mode:
  serial port -> acquisition core -> raw bin

mock mode:
  fake OE1022D -> fake RALL frame -> acquisition core -> raw bin

replay mode:
  previous raw bin -> parser -> parsed sample -> UI / tests
```

同一套 parser 和 writer 应该被 real、mock、replay 三种模式共用。

---

## 6. Inputs

### 6.1 Runtime Inputs

本模块运行时输入包括：

```text
device_id
serial port binding
baud rate
data bits
stop bits
parity
read timeout
write timeout
acquisition command
acquisition period
channel selection
field selection
run_id
step_id
resource lease
raw output path
```

### 6.2 Configuration Inputs

来自 station/profile/recipe/resolved_recipe：

```json
{
  "device_id": "oe1022d_main",
  "kind": "lockin_amplifier",
  "driver": "oe1022d",
  "transport": {
    "type": "serial",
    "port": "COM3",
    "baud_rate": 115200,
    "data_bits": 8,
    "parity": "none",
    "stop_bits": 1,
    "read_timeout_ms": 100,
    "write_timeout_ms": 100
  },
  "acquisition": {
    "command": "RALL?",
    "period_ms": 100,
    "mode": "query_response",
    "primary_channel": "B",
    "primary_field": "X",
    "record_raw": true,
    "record_parsed_ring": true,
    "ui_downsample": true
  }
}
```

### 6.3 External Inputs

来自其他模块：

| Source | Input | 说明 |
|---|---|---|
| Device Registry | OE1022D lease / connection handle | 采集开始前必须申请资源租约 |
| Executor | run_id / step_id / acquisition start-stop | 实验步骤控制采集生命周期 |
| Timing Module | settle / discard / average window | 不直接控制采集线程，但影响 sample selection |
| Data Logging | run directory / raw file path | 统一 run 目录 |
| GUI | start/stop request / display window | GUI 只能请求，不能直接读串口 |
| Harness | fake frame / replay path | 测试输入 |

---

## 7. Outputs

### 7.1 Runtime Outputs

```text
raw frame stream
parsed sample stream
latest snapshot
ring buffer window
downsampled chart data
acquisition status
acquisition events
error events
benchmark metrics
```

### 7.2 File Outputs

每个 run 目录下：

```text
raw/oe1022d.rawbin
raw/oe1022d.index.jsonl
raw/oe1022d_parse_errors.jsonl
parsed/oe1022d_samples.parquet      post-run
exports/oe1022d_export.csv          post-run only
events/events.jsonl
reports/oe1022d_benchmark.json
```

### 7.3 API Outputs

```rust
AcquisitionStatus
LatestSample
SampleWindow
DownsampledSeries
RawFrameStats
ParserStats
DropFrameStats
BenchmarkReport
```

---

## 8. Runtime Model

### 8.1 Process Boundary

第一阶段采用单 Tauri backend process 内的多线程模型：

```text
Tauri backend process
  ├── Device Manager
  ├── Executor
  ├── OE1022D Acquisition Core
  │     ├── acquisition thread
  │     ├── parser thread
  │     ├── writer thread
  │     └── downsample cache
  └── GUI command API
```

后期如果需要远程实验服务，可以把 Acquisition Core 抽为独立 daemon，但 v0.2 不做。

---

### 8.2 Thread Model

```text
[Executor]
  │ start_acquisition(run_id, step_id)
  ▼
[Acquisition Supervisor]
  ├── owns state machine
  ├── starts/stops threads
  └── emits events

[Acquisition Thread]
  ├── send RALL? / read response
  ├── timestamp raw frame
  └── push RawFrameEnvelope

[Parser Thread]
  ├── pop RawFrameEnvelope
  ├── parse fields
  ├── validate sample
  └── push ParsedSample

[Writer Thread]
  ├── append raw bytes to rawbin
  ├── append index.jsonl
  └── flush according to policy

[Downsample Service]
  ├── read parsed ring buffer
  ├── build UI window
  └── return compact series
```

---

### 8.3 Acquisition Lifecycle

```text
Idle
  ↓ configure
Configured
  ↓ acquire lease
LeaseAcquired
  ↓ start threads
Starting
  ↓ first valid frame
Running
  ↓ stop request
Stopping
  ↓ join threads + flush
Stopped
```

异常路径：

```text
Running
  ↓ serial timeout / parse flood / writer error
Recovering
  ↓ recovered
Running
```

或：

```text
Running
  ↓ fatal error / emergency stop
Faulted
  ↓ safe close
Stopped
```

---

### 8.4 State Machine

```rust
pub enum AcquisitionState {
    Idle,
    Configured,
    LeaseAcquired,
    Starting,
    Running,
    Recovering,
    Stopping,
    Stopped,
    Faulted,
}
```

状态转移必须写入 events.jsonl。

---

## 9. Acquisition Modes

### 9.1 Query-response Mode

第一阶段默认模式：

```text
loop:
  t_query = now_monotonic()
  serial.write("RALL?\n")
  read_until(frame_terminator or timeout)
  t_recv = now_monotonic()
  push raw frame
  sleep_until(next_period)
```

优点：

```text
1. 行为清晰
2. 容易 mock
3. 容易记录 query/response 时间
4. 容易 debug
```

风险：

```text
1. 查询频率过高可能打断设备内部状态
2. timeout 设置不当会导致周期抖动
3. 状态查询不能插入同一串口影响 RALL?
```

### 9.2 Continuous Read Mode

如果后续确认 OE1022D 支持连续输出模式，可新增 continuous mode：

```text
serial read loop only
no per-frame command write
frame boundary by terminator / length
```

v0.2 不默认使用 continuous mode。

### 9.3 Replay Mode

```text
rawbin + index.jsonl
→ replay source
→ parser thread
→ parsed sample stream
→ tests / UI / parquet conversion
```

Replay 必须支持：

```text
1. as-fast-as-possible replay
2. real-time replay using original timestamps
3. step-limited replay
4. parse-only replay
5. UI replay demo
```

---

## 10. Data Model

### 10.1 RawFrameEnvelope

```rust
pub struct RawFrameEnvelope {
    pub run_id: String,
    pub step_id: Option<String>,
    pub device_id: String,
    pub sequence_no: u64,
    pub command: String,
    pub t_mono_query_ns: Option<u64>,
    pub t_mono_read_start_ns: u64,
    pub t_mono_recv_ns: u64,
    pub t_wall_recv_ms: i64,
    pub read_duration_ns: u64,
    pub raw_offset: u64,
    pub raw_len: u32,
    pub raw_crc32: u32,
    pub transport_status: TransportStatus,
}
```

### 10.2 ParsedSample

```rust
pub struct ParsedSample {
    pub run_id: String,
    pub step_id: Option<String>,
    pub device_id: String,
    pub sequence_no: u64,
    pub t_mono_ns: u64,
    pub t_wall_ms: i64,
    pub channel_a: Option<LockinChannelSample>,
    pub channel_b: Option<LockinChannelSample>,
    pub aux: Option<AuxSample>,
    pub status: Oe1022dSampleStatus,
    pub source_raw_offset: u64,
    pub source_raw_len: u32,
}
```

### 10.3 LockinChannelSample

```rust
pub struct LockinChannelSample {
    pub x_vrms: Option<f64>,
    pub y_vrms: Option<f64>,
    pub r_vrms: Option<f64>,
    pub theta_deg: Option<f64>,
    pub freq_hz: Option<f64>,
    pub noise_vrms: Option<f64>,
    pub harmonic_1: Option<HarmonicSample>,
    pub harmonic_2: Option<HarmonicSample>,
    pub overload: Option<bool>,
    pub pll_locked: Option<bool>,
}
```

### 10.4 HarmonicSample

```rust
pub struct HarmonicSample {
    pub x_vrms: Option<f64>,
    pub y_vrms: Option<f64>,
    pub r_vrms: Option<f64>,
    pub theta_deg: Option<f64>,
}
```

### 10.5 AuxSample

```rust
pub struct AuxSample {
    pub adc1_v: Option<f64>,
    pub adc2_v: Option<f64>,
    pub adc3_v: Option<f64>,
    pub adc4_v: Option<f64>,
}
```

### 10.6 Sample Status

```rust
pub struct Oe1022dSampleStatus {
    pub parse_ok: bool,
    pub frame_complete: bool,
    pub timeout: bool,
    pub crc_ok: Option<bool>,
    pub value_range_warning: bool,
    pub missing_fields: Vec<String>,
    pub parser_version: String,
}
```

---

## 11. Raw Bin Format

### 11.1 文件原则

raw bin 必须满足：

```text
1. append-only
2. 可回放
3. 可部分读取
4. index.jsonl 可定位每个 frame
5. 不依赖 CSV
6. 不依赖 GUI
7. parser 更新后可重新解析
```

### 11.2 文件布局 v0.2

```text
raw/oe1022d.rawbin
  [FileHeader]
  [FrameRecord]
  [FrameRecord]
  [FrameRecord]
  ...
```

### 11.3 FileHeader

```text
magic:        b"OE1022D_RAW_V02\0"
header_len:   u32
created_ms:   i64
run_id_len:   u16
run_id:       utf8 bytes
reserved:     bytes
```

### 11.4 FrameRecord

```text
record_magic:    u32
sequence_no:     u64
t_mono_recv_ns:  u64
t_wall_recv_ms:  i64
command_len:     u16
raw_len:         u32
raw_crc32:       u32
command:         utf8 bytes
raw_payload:     bytes
```

说明：

```text
raw_payload 第一阶段可以是 ASCII response bytes。
如果后续 OE1022D 使用 binary response，则 raw_payload 保持 bytes，不改上层接口。
```

### 11.5 index.jsonl

每个 frame 一行：

```json
{
  "type": "oe1022d_raw_frame_index",
  "version": "0.2",
  "run_id": "2026-xx-xx_run_001",
  "step_id": "step_000124",
  "device_id": "oe1022d_main",
  "sequence_no": 1234,
  "command": "RALL?",
  "raw_offset": 987654,
  "raw_len": 128,
  "raw_crc32": "0x12345678",
  "t_mono_query_ns": 1234567800000,
  "t_mono_recv_ns": 1234567900000,
  "t_wall_recv_ms": 1770000000000,
  "read_duration_ns": 100000000,
  "parse_status": "ok"
}
```

---

## 12. Parser Requirements

### 12.1 Parser Contract

```rust
pub trait Oe1022dParser {
    fn parser_version(&self) -> &'static str;
    fn parse_frame(&self, frame: &RawFrameEnvelope, raw: &[u8]) -> Result<ParsedSample, ParseError>;
}
```

### 12.2 Parser Rules

```text
1. parser 不得修改 raw frame
2. parser 不得进行 I/O
3. parser 不得访问 serial port
4. parser 不得访问 GUI
5. parser 不得推断未出现字段为 0
6. parser 必须区分 missing、invalid、out_of_range、unit_unknown
7. parser 必须保留 source_raw_offset / source_raw_len
8. parser 必须带 parser_version
```

### 12.3 Numeric Parsing

OE1022D 显示和返回值可能包含：

```text
uV / mV / V
nV / uV style units
Hz / kHz
deg
scientific notation
positive / negative sign
```

parser 必须统一转换为 SI-like base fields：

```text
x_vrms / y_vrms / r_vrms / noise_vrms -> volts rms
freq_hz -> Hz
theta_deg -> degree
aux -> volts
```

### 12.4 Field Mapping

第一阶段必须支持：

```text
A-X, A-Y, A-R, A-theta, A-Freq, A-Noise
B-X, B-Y, B-R, B-theta, B-Freq, B-Noise
A harmonic 1 / 2 if present
B harmonic 1 / 2 if present
AUX ADC 1-4 if present
PLL locked if available from status query or frame
input overload / gain overload if available
```

### 12.5 Parse Failure

```rust
pub enum ParseErrorKind {
    EmptyFrame,
    IncompleteFrame,
    UnknownFormat,
    MissingRequiredField,
    InvalidNumber,
    InvalidUnit,
    OutOfRange,
    EncodingError,
}
```

Parse failure 必须进入：

```text
raw/oe1022d_parse_errors.jsonl
events/events.jsonl
parser metrics
```

---

## 13. Ring Buffer

### 13.1 Buffer Types

本模块至少需要三个 buffer：

```text
raw_frame_queue         acquisition thread -> writer/parser
parsed_sample_ring      parser -> UI / executor / downsample
downsample_cache        parsed ring -> GUI chart
```

### 13.2 raw_frame_queue

要求：

```text
1. 有界队列
2. backpressure 可观测
3. 溢出时有明确策略
4. 不 silent drop
```

### 13.3 parsed_sample_ring

要求：

```text
1. 保留最近 N 秒或 N 点
2. GUI 查询不会阻塞 parser
3. Executor 可读取当前 step 的最新 sample
4. Downsample service 可读取 window
```

建议默认：

```text
capacity_samples = max(60_000, acquisition_hz * 600 seconds)
```

### 13.4 Drop Policy

优先级：

```text
1. 不丢 raw frame
2. UI ring 可以覆盖旧 parsed sample
3. downsample cache 可以重建
4. raw writer 如果失败必须进入 fault 或暂停采集
```

允许丢弃：

```text
UI-only downsample points
old parsed samples outside retention window
mock-mode generated frames when configured
```

禁止悄悄丢弃：

```text
raw frame
index entry
parse error
fatal serial error
```

---

## 14. UI Query Interface

### 14.1 GUI 允许调用

```rust
fn get_acquisition_status(device_id: &str) -> AcquisitionStatus;
fn get_latest_sample(device_id: &str) -> Option<ParsedSample>;
fn get_sample_window(query: SampleWindowQuery) -> SampleWindow;
fn get_downsampled_series(query: DownsampleQuery) -> DownsampledSeries;
fn get_parser_stats(device_id: &str) -> ParserStats;
fn get_drop_stats(device_id: &str) -> DropStats;
```

### 14.2 GUI 禁止调用

```text
serial.write
serial.read
RALL?
rawbin append
parser internals
blocking acquisition loop
```

### 14.3 DownsampleQuery

```rust
pub struct DownsampleQuery {
    pub device_id: String,
    pub channel: ChannelId,
    pub field: LockinField,
    pub t_start_mono_ns: Option<u64>,
    pub t_end_mono_ns: Option<u64>,
    pub max_points: usize,
    pub method: DownsampleMethod,
}
```

### 14.4 DownsampleMethod

```rust
pub enum DownsampleMethod {
    Latest,
    MinMaxEnvelope,
    Lttb,
    MeanBucket,
    RawIfSmall,
}
```

第一阶段 GUI 图表默认：

```text
method = MinMaxEnvelope
max_points = 1000 - 5000
refresh_rate = 10 - 30 Hz
```

---

## 15. Timing and Synchronization

### 15.1 采集周期

配置字段：

```json
{
  "period_ms": 100,
  "jitter_warn_ms": 20,
  "timeout_ms": 100,
  "max_consecutive_timeout": 5
}
```

注意：

```text
period_ms 不是锁相时间常数。
period_ms 只是软件读取间隔。
OE1022D time constant 由 profile/recipe 配置。
采样窗口选择由 Timing/Averaging PRD 定义。
```

### 15.2 timestamp 字段

每个 sample 需要携带：

```text
t_mono_query_ns
t_mono_recv_ns
t_wall_recv_ms
read_duration_ns
sequence_no
step_id
```

### 15.3 dwell / settle / average

本模块不决定哪些点进入平均，只提供带时间戳的数据。

Timing 模块可基于：

```text
SMB dwell time
mag settle time
laser settle time
OE1022D time constant
front discard window
average window
```

选择有效 sample。

---

## 16. Status Query Rules

OE1022D 同一个 serial session 上不能随意插入状态查询，避免打断 RALL? 节奏。

### 16.1 禁止行为

```text
1. GUI 每 50 ms 查询一次 OE1022D 状态
2. Executor 在采集线程运行时直接 serial.write 状态命令
3. AI agent 运行中查询 OE1022D
4. profile editor 在 running 状态修改仪器参数
```

### 16.2 允许行为

```text
1. 采集开始前执行一次 configuration snapshot
2. 采集停止后执行一次 status snapshot
3. 采集中通过 acquisition supervisor 安排低频 status query
4. status query 必须进入 command queue，不得直接插队
```

### 16.3 推荐状态查询频率

第一阶段：

```text
running status query = disabled by default
```

仅在 debug mode：

```text
status_query_period_s >= 5
```

---

## 17. Error Handling

### 17.1 Error Categories

```rust
pub enum AcquisitionErrorKind {
    SerialOpenFailed,
    SerialWriteFailed,
    SerialReadTimeout,
    SerialReadFailed,
    FrameTooShort,
    FrameTooLarge,
    ParseFailed,
    WriterFailed,
    RingBufferOverflow,
    DeviceNotLocked,
    LeaseLost,
    EmergencyStop,
}
```

### 17.2 Timeout Policy

```text
single timeout:
  record warning
  continue if below threshold

consecutive timeout >= threshold:
  enter Recovering
  attempt soft recovery

recovery failed:
  enter Faulted
  flush writer
  release lease
  notify executor
```

### 17.3 Soft Recovery

Soft recovery 可以包含：

```text
1. clear serial input buffer
2. wait recovery_delay_ms
3. send one known harmless query if allowed
4. restart acquisition loop
```

禁止：

```text
1. 自动改 OE1022D 灵敏度
2. 自动改参考源
3. 自动重置设备
4. 自动切换 Ch-A/Ch-B
5. 自动修改 recipe
```

### 17.4 Writer Error

raw writer 错误是高严重度错误。

```text
if raw writer failed:
  stop acquisition or enter fault
  emit fatal event
  do not continue silently
```

原因：如果不能记录 raw，实验不可追溯。

---

## 18. Safety Rules

OE1022D 本身不是高危险输出设备，但错误采集仍会导致实验数据不可用，或掩盖真实过载状态。

### 18.1 Safety-relevant Conditions

```text
1. PLL unlocked
2. input overload
3. gain overload
4. reference frequency lost
5. reference frequency drift too large
6. R near noise floor unexpectedly
7. Ch-B disconnected / missing signal
8. parser unknown format
9. raw writer stopped
10. acquisition jitter too large
```

### 18.2 Safety Events

这些条件必须作为事件上报，而不是仅显示在 GUI：

```json
{
  "type": "oe1022d_warning",
  "severity": "warning",
  "device_id": "oe1022d_main",
  "run_id": "run_001",
  "step_id": "step_0012",
  "code": "PLL_UNLOCKED",
  "message": "Ch-B PLL is not locked during acquisition",
  "t_mono_ns": 123456789,
  "t_wall_ms": 1770000000000
}
```

### 18.3 Safety Interaction

本模块可以向 Safety/Interlock 上报：

```text
oe1022d_overload
pll_unlocked
writer_failed
frame_timeout_flood
```

但是否 emergency stop 由 Safety/Interlock 和 Executor 决定。

---

## 19. API / Interface

### 19.1 Rust Crate Layout

```text
crates/oe1022d-core/
  src/
    lib.rs
    config.rs
    acquisition.rs
    parser.rs
    rawbin.rs
    replay.rs
    ring.rs
    downsample.rs
    status.rs
    error.rs
    mock.rs
    metrics.rs
  tests/
    parser_golden.rs
    raw_replay.rs
    mock_acquisition.rs
    timing_jitter.rs
```

### 19.2 Public API

```rust
pub struct Oe1022dAcquisitionCore;

impl Oe1022dAcquisitionCore {
    pub fn configure(config: Oe1022dAcqConfig) -> Result<Self>;
    pub fn start(&self, run_ctx: RunContext, lease: DeviceLease) -> Result<()>;
    pub fn stop(&self) -> Result<StopReport>;
    pub fn status(&self) -> AcquisitionStatus;
    pub fn latest_sample(&self) -> Option<ParsedSample>;
    pub fn sample_window(&self, query: SampleWindowQuery) -> Result<SampleWindow>;
    pub fn downsampled_series(&self, query: DownsampleQuery) -> Result<DownsampledSeries>;
}
```

### 19.3 Executor Interface

Executor 只调用高层接口：

```text
start_acquisition
mark_step_context
stop_acquisition
get_status
```

Executor 不调用 parser，不直接访问 serial。

### 19.4 Data Logging Interface

```rust
pub trait Oe1022dRawSink {
    fn open(run_dir: &Path, metadata: RawFileMetadata) -> Result<Self>;
    fn append_frame(&mut self, frame: &RawFrameEnvelope, raw: &[u8]) -> Result<RawWriteResult>;
    fn flush(&mut self) -> Result<()>;
    fn close(self) -> Result<RawCloseReport>;
}
```

---

## 20. Configuration Schema Draft

```json
{
  "$schema": "https://odor-ctl-web/schemas/oe1022d_acquisition.schema.json",
  "type": "object",
  "required": ["device_id", "mode", "command", "period_ms", "primary_channel", "primary_field"],
  "properties": {
    "device_id": { "type": "string" },
    "mode": {
      "type": "string",
      "enum": ["query_response", "continuous", "mock", "replay"]
    },
    "command": {
      "type": "string",
      "default": "RALL?"
    },
    "period_ms": {
      "type": "number",
      "minimum": 10,
      "default": 100
    },
    "read_timeout_ms": {
      "type": "number",
      "minimum": 1,
      "default": 100
    },
    "max_consecutive_timeout": {
      "type": "integer",
      "minimum": 1,
      "default": 5
    },
    "primary_channel": {
      "type": "string",
      "enum": ["A", "B"]
    },
    "primary_field": {
      "type": "string",
      "enum": ["X", "Y", "R", "theta", "freq", "noise"]
    },
    "record_raw": { "type": "boolean", "default": true },
    "record_index": { "type": "boolean", "default": true },
    "parsed_ring_capacity": { "type": "integer", "minimum": 1000 },
    "ui_downsample": { "type": "boolean", "default": true },
    "debug_status_query_period_s": {
      "type": ["number", "null"],
      "default": null
    }
  }
}
```

---

## 21. Mock and Harness

### 21.1 Fake OE1022D

必须实现 fake OE1022D：

```text
1. 接收 RALL? 命令
2. 返回合法 mock frame
3. 可配置 noise level
4. 可配置 frequency
5. 可配置 PLL locked/unlocked
6. 可配置 timeout probability
7. 可配置 malformed frame probability
8. 可配置 overload flag
```

### 21.2 Mock Frame Generator

```rust
pub struct MockOe1022dFrameConfig {
    pub channel_b_x_vrms: f64,
    pub channel_b_y_vrms: f64,
    pub channel_b_r_vrms: f64,
    pub channel_b_theta_deg: f64,
    pub channel_b_freq_hz: f64,
    pub channel_b_noise_vrms: f64,
    pub noise_std: f64,
    pub malformed_rate: f64,
    pub timeout_rate: f64,
}
```

### 21.3 Required Tests

```text
parser golden tests
mock acquisition 1 min test
raw bin write/read test
replay test
timeout recovery test
malformed frame test
ring buffer overflow test
UI query non-blocking test
benchmark 30 min dry run
real hardware smoke test
```

### 21.4 Hardware-in-loop Tests

有真实设备时必须支持：

```text
cargo test --features hardware -- --ignored oe1022d_real_smoke
```

该测试必须：

```text
1. 使用配置文件指定端口
2. 打开真实 OE1022D
3. 查询 IDN / device info if available
4. 执行短时间 RALL? 采集
5. 保存 raw bin
6. replay raw bin
7. 比较 parsed sample 是否可用
```

---

## 22. Benchmark Requirements

### 22.1 Benchmark Report

每次 benchmark 输出：

```json
{
  "type": "oe1022d_benchmark_report",
  "version": "0.2",
  "duration_s": 1800,
  "target_period_ms": 100,
  "frames_total": 18000,
  "frames_ok": 17998,
  "frames_timeout": 1,
  "frames_parse_failed": 1,
  "mean_period_ms": 100.1,
  "p95_period_ms": 104.0,
  "p99_period_ms": 110.0,
  "max_period_ms": 145.0,
  "raw_bytes_written": 12345678,
  "writer_errors": 0,
  "ring_overflows": 0,
  "ui_query_p95_ms": 3.0
}
```

### 22.2 Acceptance Benchmarks

v0.2 验收目标：

```text
1. mock mode 连续 30 min 无 crash
2. real mode 连续 30 min 可采集
3. UI 10-30 Hz 查询不影响采集
4. raw bin 可 replay
5. parser 结果和旧 Python 对照一致
6. frame period jitter 可量化
7. writer failure 不 silent
8. no real-time CSV
```

---

## 23. Interaction with Other PRDs

### 23.1 与 02 Device Registry

```text
Device Registry 负责连接和资源租约。
OE1022D Acquisition Core 负责拿到 lease 后运行采集。
```

禁止：

```text
Acquisition Core 自己扫描串口并抢占设备。
```

### 23.2 与 04 Recipe Schema

Recipe 中可以指定：

```text
primary_channel
primary_field
period_ms
time_constant expectation
record fields
```

但 recipe 不能指定危险底层操作，例如直接发送任意 OE1022D 命令。

### 23.3 与 05 Compiler & Executor

Compiler 生成 resolved_recipe。  
Executor 按 step 调用 acquisition start/stop/mark_step。

Acquisition Core 不展开 sweep。

### 23.4 与 06 Timing / Sync / Averaging

Timing 模块基于 timestamp 选择有效数据窗口。

Acquisition Core 只提供带时间戳的连续 sample。

### 23.5 与 07 Data Logging

Data Logging 定义 run directory 和统一文件布局。  
OE1022D Core 实现 rawbin writer 或调用 data logging sink。

### 23.6 与 08 GUI

GUI 只能调用 downsample API。

### 23.7 与 10 Safety

OE1022D Core 上报 overload、PLL unlocked、writer failed、timeout flood 等事件。

### 23.8 与 11 Harness

Harness 必须提供 fake OE1022D、fake RALL frame、raw replay tests。

### 23.9 与 12 Agent Workflow

Agent 可以实现 parser、mock、tests，但不能越权修改 GUI、SMB100A、recipe schema。

---

## 24. Implementation Plan

### 24.1 Phase 0: Parser Prototype

交付：

```text
parser.rs
sample structs
mock raw frames
golden parser tests
```

目标：

```text
能把已知 OE1022D frame 解析为 ParsedSample。
```

### 24.2 Phase 1: Raw Bin and Replay

交付：

```text
rawbin.rs
index.jsonl writer
replay.rs
raw replay tests
```

目标：

```text
无需真实设备，也能跑完整 parser/replay 流水线。
```

### 24.3 Phase 2: Mock Acquisition

交付：

```text
fake OE1022D
mock acquisition loop
mock benchmark
```

目标：

```text
mock mode 30 min 稳定运行。
```

### 24.4 Phase 3: Real Serial Acquisition

交付：

```text
serial transport integration
real smoke test
30 min real benchmark
```

目标：

```text
真实 OE1022D 连续采集并生成 raw bin。
```

### 24.5 Phase 4: UI Query Integration

交付：

```text
downsample service
Tauri command wrapper
chart query demo
```

目标：

```text
GUI 10-30 Hz 刷新不影响采集。
```

---

## 25. Acceptance Criteria

### 25.1 Functional Acceptance

```text
[ ] 能以 mock mode 启动采集
[ ] 能以 real mode 启动采集
[ ] 能配置 RALL? 采集周期
[ ] 能记录 raw frame size
[ ] 能保存 raw bin
[ ] 能保存 index.jsonl
[ ] 能解析 Ch-B X/Y/R/theta/Freq/Noise
[ ] 能保留 Ch-A 扩展能力
[ ] 能生成 latest sample
[ ] 能生成 sample window
[ ] 能生成 downsampled series
[ ] 能 replay raw bin
[ ] 能输出 benchmark report
```

### 25.2 Performance Acceptance

```text
[ ] 连续采集 30 min 不崩溃
[ ] frame rate 稳定并可量化
[ ] UI 10-30 Hz 查询不影响采集
[ ] raw writer 无 silent failure
[ ] parser failure 不影响 raw 保存
[ ] ring buffer overflow 有事件记录
```

### 25.3 Correctness Acceptance

```text
[ ] parser golden tests 通过
[ ] mock frame roundtrip 通过
[ ] raw replay parsed sample 与原 sample 一致
[ ] 与旧 Python 采集结果在同一测试数据上字段一致
[ ] 单位转换正确
[ ] timestamp 单调递增
```

### 25.4 Architecture Acceptance

```text
[ ] GUI 不直接读 RALL?
[ ] Web 前端不解析 binary frame
[ ] 不实时写 CSV
[ ] 采集线程不画图
[ ] 采集线程不做复杂分析
[ ] 状态查询不打断 RALL? 主循环
[ ] Python 不做 OE1022D 高频采集
[ ] AI agent 不直接控制 OE1022D
```

---

## 26. Agent Constraints

### 26.1 OE1022D Core Agent 允许做

```text
1. 实现 Rust parser
2. 实现 rawbin writer
3. 实现 replay tool
4. 实现 mock frame generator
5. 实现 acquisition thread skeleton
6. 实现 parser tests
7. 实现 benchmark report
8. 读 OE1022D manual / panel docs
9. 补充字段映射表
10. 提交 PR 修改 oe1022d-core crate
```

### 26.2 OE1022D Core Agent 禁止做

```text
1. 修改 GUI 页面结构
2. 修改 recipe schema 顶层结构
3. 控制 SMB100A
4. 控制 laser
5. 控制 mag axes
6. 修改 safety limit
7. 添加 AI 直接硬件控制入口
8. 把 CSV 作为实时采集格式
9. 在前端解析 raw frame
10. 绕过 Device Registry 直接抢占串口
```

### 26.3 Review Checklist

任何 agent 提交本模块代码前必须回答：

```text
[ ] 是否保留 raw frame？
[ ] parser 失败时 raw 是否仍可回放？
[ ] GUI 是否没有直接访问 serial？
[ ] 是否没有实时 CSV？
[ ] 是否有 mock frame？
[ ] 是否有 parser golden test？
[ ] 是否有 replay test？
[ ] 是否记录 timestamp？
[ ] 是否记录 frame sequence_no？
[ ] 是否有 benchmark 输出？
[ ] 是否没有改 SMB100A / laser / mag？
```

---

## 27. Open Questions

以下问题留待 v0.3 或真实设备测试确认：

```text
1. RALL? 的真实返回格式是否稳定？
2. RALL? 是否有固定 terminator？
3. RALL? 的最短稳定采集周期是多少？
4. OE1022D 是否支持连续输出模式？
5. 真实设备在 50 ms / 100 ms / 200 ms period 下 jitter 如何？
6. status query 是否会影响 RALL? 主循环？
7. overload / PLL lock 是否能在 RALL? frame 中直接读到？
8. 是否需要单独命令读取 PLL / overload 状态？
9. Ch-B Freq/Noise 的返回单位是否与面板一致？
10. 旧 Python 采集数据字段映射是否有历史兼容问题？
```

---

## 28. v0.2 Final Decision

本模块 v0.2 决策如下：

```text
1. OE1022D 高频采集核心使用 Rust 实现。
2. 第一阶段默认 query-response RALL? 采集模式。
3. 采集线程只负责读取 raw frame 和 timestamp。
4. parser、writer、downsample 分离线程/任务。
5. 实时数据必须先写 raw bin + index.jsonl。
6. CSV 禁止作为实时采集格式。
7. GUI 禁止直接读 RALL? 或解析 raw frame。
8. UI 只通过 downsample API 获取曲线数据。
9. mock / replay / benchmark 是本模块的强制交付物。
10. 是否提高采集频率必须由 benchmark 决定，而不是主观设置。
```


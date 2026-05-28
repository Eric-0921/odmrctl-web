# ADR-002: 使用 Rust 实现 OE1022D 高性能采集核心

## Status

Accepted

## Date

2026-05-28

## Decision Owner

ODMR Automation Project

## Related Documents

```text
docs/prd/00_main_prd.md
docs/prd/01_architecture_prd.md
docs/prd/03_oe1022d_acquisition_prd.md
docs/prd/06_timing_sync_averaging_prd.md
docs/prd/07_data_logging_prd.md
docs/prd/08_gui_tauri_chart_prd.md
docs/prd/10_safety_interlock_prd.md
docs/prd/11_harness_mock_replay_prd.md
docs/prd/12_agent_workflow_prd.md

docs/adr/ADR-001-tauri-ui.md
docs/adr/ADR-003-json-recipe-driven.md
docs/adr/ADR-004-no-ai-live-hardware.md
docs/adr/ADR-005-raw-bin-before-csv.md
```

---

## 1. Context

本项目需要从旧式 ODMR 控制程序重构为：

```text
轻 GUI + JSON recipe + Rust 高性能采集核心 + Python/AI 生成和检查实验方案
```

其中 OE1022D 锁相放大器的数据采集是第一阶段最关键的实时路径之一。OE1022D 采集模块必须承担：

```text
RALL? 采集循环
raw frame 读取
binary frame parser
ring buffer
raw bin 落盘
index / events 记录
UI 查询接口
错误恢复
drop frame 策略
raw replay
benchmark
```

该模块不能再由 GUI 线程、Web 前端或临时 Python 脚本承担。核心原因是：

```text
GUI 卡顿不应影响采集
前端不能解析二进制 frame
实时采集不能直接写 CSV
状态查询不能打断 RALL? 采集循环
采集线程不能画图
采集线程不能做复杂分析
AI agent 不能直接控制硬件
```

旧系统如果继续使用 Python + GUI 混合结构，容易出现以下问题：

```text
串口读取、解析、绘图、保存互相阻塞
采集时间戳不稳定
UI 刷新频率影响采集频率
raw 数据不可完整回放
异常恢复路径无法稳定测试
多 agent 开发时没有统一 runtime contract
```

因此，需要把 OE1022D 的实时采集路径单独下沉为 Rust core crate，使其成为一个独立、可测试、可 replay、可 benchmark 的采集核心。

---

## 2. Decision

决定使用 Rust 实现 OE1022D 高性能采集核心，形成独立 crate：

```text
oe1022d-core
```

该 crate 是 OE1022D 实时采集的唯一核心实现，负责：

```text
真实串口连接
mock 串口连接
RALL? 请求循环
raw frame 接收
frame 边界识别
binary parser
parsed sample 输出
monotonic timestamp 记录
raw bin 写入
index.jsonl 写入
events.jsonl 写入
ring buffer 暴露
replay tool 支持
parser unit tests
benchmark tests
```

GUI、Python、AI agent 都不得绕过 `oe1022d-core` 直接读取 OE1022D 的实时数据流。

系统边界如下：

```text
OE1022D device / fake OE1022D
        ↓ serial / mock transport
oe1022d-core
        ↓ parsed sample stream + downsampled trace + status snapshot
Rust backend / Tauri command boundary
        ↓
Web GUI chart / status panel
```

---

## 3. Decision Details

### 3.1 crate 职责

`oe1022d-core` 负责：

```text
1. Transport
   - 打开和关闭串口
   - 设置 baud rate / timeout / terminator
   - 支持真实串口和 mock transport
   - 支持 connection status 查询
   - 支持 safe disconnect

2. Acquisition Loop
   - 周期性发送 RALL?
   - 读取 raw frame
   - 记录 command_sent_monotonic_ts
   - 记录 command_returned_monotonic_ts
   - 控制采集周期和超时
   - 避免其他状态查询打断采集循环

3. Parser
   - 识别 raw frame 边界
   - 校验 frame 长度
   - 解析 X / Y / R / theta / freq / noise 等字段
   - 输出强类型 ParsedSample
   - 对坏 frame 生成 error event

4. Buffer
   - 维护 ring buffer
   - 为 UI 提供最近窗口数据
   - 为实时曲线提供降采样数据
   - 不把 UI 订阅者放进采集热路径

5. Recorder
   - 实时写 raw bin
   - 实时写 index.jsonl
   - 实时写 events.jsonl
   - 实验后再转换 parquet / csv

6. Replay
   - 从 raw bin + index 重放采集流
   - 支持 parser regression test
   - 支持 UI mock run
   - 支持 benchmark

7. Metrics
   - frame rate
   - dropped frame count
   - bad frame count
   - read timeout count
   - parser latency
   - recorder latency
   - ring buffer occupancy
```

### 3.2 crate 不负责

`oe1022d-core` 不负责：

```text
GUI 绘图
Web 前端状态管理
recipe schema
recipe compiler
recipe executor 的完整流程编排
SMB100A 控制
Laser 控制
Mag X/Y/Z 控制
AI prompt / agent workflow
复杂数据分析
CSV 导出格式设计
parquet 离线转换
实验谱线拟合
```

### 3.3 推荐内部模块结构

```text
oe1022d-core/
  src/
    lib.rs
    config.rs
    error.rs
    transport/
      mod.rs
      serial.rs
      mock.rs
    protocol/
      mod.rs
      commands.rs
      frame.rs
    acquisition/
      mod.rs
      loop.rs
      controller.rs
    parser/
      mod.rs
      rall.rs
    buffer/
      mod.rs
      ring.rs
      downsample.rs
    recorder/
      mod.rs
      raw_bin.rs
      index_jsonl.rs
      events_jsonl.rs
    replay/
      mod.rs
      reader.rs
    metrics/
      mod.rs
    api/
      mod.rs
      snapshot.rs
  tests/
    parser_tests.rs
    replay_tests.rs
    mock_acquisition_tests.rs
    timestamp_tests.rs
  benches/
    parser_bench.rs
    acquisition_bench.rs
```

---

## 4. Runtime Model

### 4.1 线程模型

第一阶段采用明确的多线程模型，而不是把所有逻辑塞进单个 async runtime。

推荐模型：

```text
Acquisition Thread
    - 独占 OE1022D transport
    - 发送 RALL?
    - 读取 raw frame
    - 给每个 frame 加 monotonic timestamp
    - 把 raw frame 推给 parser/recorder 通道

Parser Thread
    - 消费 raw frame
    - 解析为 ParsedSample
    - 写入 ring buffer
    - 生成 parser error event

Recorder Thread
    - 消费 raw frame + metadata
    - 写 raw bin
    - 写 index.jsonl
    - 写 events.jsonl

Status / Metrics Path
    - 周期性汇总 frame rate / drop count / timeout count
    - 给 GUI 提供 snapshot

GUI Thread / Web Frontend
    - 只读取 snapshot 和 downsampled trace
    - 不直接读串口
    - 不直接解析 raw frame
```

### 4.2 为什么第一阶段不用全 async

第一阶段不采用“全 async 串口采集模型”，原因是：

```text
OE1022D 采集是单设备、单串口、顺序命令响应模型
blocking serial thread 更容易验证时序和故障恢复
线程边界比 async task 边界更容易被多 agent 团队理解和测试
benchmark 与 replay 更容易稳定复现
```

后续如果需要多台 OE1022D 或更复杂的 IO 复用，可以再写新的 ADR 评估 async runtime。

---

## 5. Data Model

### 5.1 RawFrame

```rust
pub struct RawFrame {
    pub seq: u64,
    pub command: CommandKind,
    pub command_sent_monotonic_ns: u128,
    pub command_returned_monotonic_ns: u128,
    pub wall_time_unix_ms: i64,
    pub bytes: Vec<u8>,
}
```

### 5.2 ParsedSample

```rust
pub struct ParsedSample {
    pub seq: u64,
    pub monotonic_ns: u128,
    pub wall_time_unix_ms: i64,
    pub channel: OeChannel,
    pub x_vrms: f64,
    pub y_vrms: f64,
    pub r_vrms: f64,
    pub theta_deg: f64,
    pub frequency_hz: f64,
    pub noise_vrms: Option<f64>,
    pub raw_frame_offset: u64,
}
```

### 5.3 AcquisitionEvent

```rust
pub struct AcquisitionEvent {
    pub ts_monotonic_ns: u128,
    pub wall_time_unix_ms: i64,
    pub level: EventLevel,
    pub kind: EventKind,
    pub message: String,
    pub seq: Option<u64>,
}
```

### 5.4 Run Directory 输出

`oe1022d-core` 在实验运行中参与写入：

```text
run_directory/
  raw/
    oe1022d_rall.rawbin
  index.jsonl
  events.jsonl
  station_snapshot.json
  resolved_recipe.json
```

实验结束后由离线转换工具生成：

```text
parsed.parquet
export.csv
```

---

## 6. API / Interface

### 6.1 Rust API 草案

```rust
pub struct Oe1022dCore;

impl Oe1022dCore {
    pub fn open(config: Oe1022dConfig) -> Result<Self>;
    pub fn start_acquisition(&self, plan: AcquisitionPlan) -> Result<()>;
    pub fn stop_acquisition(&self) -> Result<()>;
    pub fn snapshot(&self) -> Oe1022dSnapshot;
    pub fn recent_samples(&self, window: SampleWindow) -> Vec<ParsedSample>;
    pub fn downsampled_trace(&self, request: TraceRequest) -> TraceResponse;
    pub fn disconnect_safe(&self) -> Result<()>;
}
```

### 6.2 GUI 可见接口

Tauri 后端可以向前端暴露：

```text
connect_oe1022d(config)
disconnect_oe1022d()
start_oe1022d_acquisition(plan)
stop_oe1022d_acquisition()
get_oe1022d_status()
get_oe1022d_trace(window, max_points)
get_oe1022d_metrics()
```

前端不得暴露或调用：

```text
send_raw_command("RALL?")
read_serial_bytes()
parse_rall_frame(bytes)
write_raw_bin(bytes)
```

---

## 7. Safety and Reliability Rules

### 7.1 采集安全规则

```text
采集开始前必须确认 device registry 中 OE1022D 连接状态为 connected
采集开始前必须确认 run directory 已创建并可写
采集过程中 acquisition thread 独占 transport
采集过程中禁止 GUI 发送任意 OE1022D 原始命令
采集过程中禁止状态查询打断 RALL? 循环
读取超时必须记录 event
bad frame 必须记录 event
parser error 不得导致进程 panic
recorder error 必须触发 run error 或 safe stop
stop 必须可在有限时间内完成
safe disconnect 必须停止采集线程后再关闭 transport
```

### 7.2 数据可靠性规则

```text
raw bin 是实时主数据
index.jsonl 是 raw bin 的定位索引
parsed sample 是派生数据
csv 只是导出格式
每个 parsed sample 必须能回溯到 raw frame offset
每个 raw frame 必须能回溯到 seq 和 timestamp
每个异常必须进入 events.jsonl
```

### 7.3 Rust 代码约束

```text
第一阶段 oe1022d-core 禁止 unsafe
错误必须使用 Result 返回，不允许采集热路径 panic
所有 public struct 必须可序列化或可转换为 snapshot DTO
parser 必须纯函数化，便于 unit test
transport 必须 trait 化，便于 mock
```

---

## 8. Alternatives Considered

### 8.1 继续使用 Python 采集

结论：Reject。

优点：

```text
开发快
已有实验脚本可复用
pyserial / numpy / pandas 生态成熟
```

拒绝原因：

```text
难以保证 UI 卡顿不影响采集
长期运行时性能和错误恢复路径不够明确
线程、串口、绘图、保存容易混在一起
不适合作为多 agent 开发的稳定核心边界
高频采集和 raw replay 测试不够工程化
```

Python 仍然保留在系统中，但定位为：

```text
recipe 生成
离线分析
parquet / csv 后处理
notebook
AI agent 工具层
```

### 8.2 Web 前端直接读串口

结论：Reject。

拒绝原因：

```text
违反 GUI 不直接访问硬件原则
前端不应解析 binary frame
安全边界无法集中在 Rust 后端
浏览器/WebView 权限模型不适合作为实验硬件控制核心
测试 harness 难以覆盖真实运行路径
```

### 8.3 C / C++ 实现采集核心

结论：Reject for Phase 1。

优点：

```text
性能强
硬件控制经验成熟
可实现非常低层的 IO 优化
```

拒绝原因：

```text
内存安全和并发错误成本更高
与 Tauri/Rust 后端集成需要 FFI 边界
多 agent 开发时更容易出现接口和构建复杂度
第一阶段没有必须使用 C/C++ 的性能理由
```

### 8.4 Rust 全 async runtime

结论：Defer。

优点：

```text
可扩展到多设备 IO
适合高并发网络和异步任务
```

暂缓原因：

```text
OE1022D 第一阶段是单串口顺序采集
blocking acquisition thread 更直接
时序、replay、benchmark 更容易验证
避免把 runtime 复杂度过早引入核心路径
```

后续如果出现以下需求，可以重新评估：

```text
多台 OE1022D 同时采集
多个串口设备高频并发
远程网络采集服务
跨进程数据流调度
```

---

## 9. Consequences

### 9.1 Positive Consequences

```text
采集路径和 GUI 解耦
raw 数据可完整回放
parser 可独立测试
mock / fake RALL frame 可覆盖无设备场景
run 过程中 UI 卡顿不影响采集线程
时间戳模型更稳定
错误和 drop frame 可被量化
后续性能优化有明确 benchmark
```

### 9.2 Negative Consequences

```text
Rust 开发成本高于 Python 脚本
需要维护 Rust crate API
需要写 mock transport 和 replay tool
需要前期定义 raw frame / index / event 格式
需要额外处理 Rust 与 Tauri 前端的 DTO 转换
```

### 9.3 Risk Mitigation

```text
先实现 mock transport，再接真实设备
先实现 parser unit tests，再接 acquisition loop
先实现 raw replay，再做 GUI 实时曲线
先完成 30 min 稳定性测试，再进入复杂 recipe executor 集成
保留 Python 离线分析，不把所有分析逻辑强行迁移到 Rust
```

---

## 10. Acceptance Criteria

ADR-002 对应的实现必须满足：

```text
1. oe1022d-core crate 可以独立编译和测试
2. parser tests 覆盖正常 frame、坏 frame、截断 frame、异常数值
3. mock RALL frame 可以在无设备环境下跑完整采集流程
4. raw bin 可以通过 replay tool 重放
5. 连续采集 30 min 不崩溃
6. frame rate 稳定，抖动有 metrics 记录
7. UI 卡顿或停止刷新不影响 acquisition thread
8. recorder 写入失败会触发明确错误并进入 events.jsonl
9. stop / safe disconnect 在有限时间内完成
10. 解析结果与旧 Python 结果在允许误差内一致
11. raw bin + index.jsonl 可以定位任意 parsed sample 的原始 frame
12. benchmark report 记录 parser latency、frame rate、drop count
```

---

## 11. Agent Constraints

### 11.1 OE1022D Core Agent 可以做

```text
实现 oe1022d-core crate
实现 RALL? parser
实现 mock RALL frame
实现 ring buffer
实现 raw bin recorder
实现 replay tool
实现 parser tests
实现 benchmark report
实现 Tauri 后端可调用的 Rust API
```

### 11.2 OE1022D Core Agent 禁止做

```text
不改 GUI 页面布局
不改 recipe schema
不碰 SMB100A 控制逻辑
不碰 Laser 控制逻辑
不碰 Mag X/Y/Z 控制逻辑
不实现 AI agent prompt
不把 CSV 作为实时采集格式
不让 GUI 线程直接读 RALL?
不在采集线程中绘图
不在采集线程中做谱线拟合
```

### 11.3 Review Checklist

```text
是否保持 GUI 不直接访问硬件？
是否保持 AI 不直接访问硬件？
是否保留 raw bin 作为实时主数据？
是否有 mock transport？
是否有 replay test？
是否有 timestamp test？
是否有 bad frame test？
是否有 30 min stability test？
是否有 benchmark report？
是否没有 unsafe？
是否没有采集热路径 panic？
```

---

## 12. External References

这些参考只用于支持工程判断，不替代本项目 PRD 和 safety rule。

```text
Rust Book - Fearless Concurrency
https://doc.rust-lang.org/book/ch16-00-concurrency.html

Rust std::time::Instant
https://doc.rust-lang.org/std/time/struct.Instant.html

serialport-rs crate documentation
https://docs.rs/serialport/latest/serialport/
```

---

## 13. Final Decision Summary

最终决定：

```text
OE1022D 实时采集核心使用 Rust 实现。
```

第一阶段交付目标是：

```text
oe1022d-core crate
mock RALL frame
parser tests
raw bin recorder
raw replay tool
benchmark report
30 min stability test
```

这项决策的核心目的不是“为了用 Rust 而用 Rust”，而是为了把 ODMR 系统中最容易出问题的实时采集路径变成一个：

```text
边界清楚
可测试
可回放
可 benchmark
不受 GUI 卡顿影响
不被 AI agent 直接触碰
不依赖 CSV 实时写入
```

的工程核心。

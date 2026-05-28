# Sub-PRD 06: Timing / Synchronization / Averaging PRD v0.2

> 文件定位：Timing / Synchronization / Averaging Sub-PRD  
> 所属项目：odor-ctl-web / ODMR Automation  
> 上级文档：00_main_prd.md v0.2、01_architecture_prd.md v0.2、03_oe1022d_acquisition_prd.md v0.2、04_recipe_json_schema_prd.md v0.2、05_recipe_compiler_executor_prd.md v0.2  
> 当前阶段：架构冻结前的 v0.2 草案  
> 目标读者：Executor 开发者、OE1022D 采集开发者、SMB100A 驱动开发者、磁场模块开发者、数据处理开发者、GUI 开发者、AI/coding agent、实验操作者  

---

## 0. 文档目的

本文件定义 ODMR 自动化系统中的 **Timing / Synchronization / Averaging** 模块。

该模块负责把实验流程中的时间相关概念统一成可计算、可记录、可回放、可审计的数据模型，包括：

```text
monotonic timestamp
wall timestamp
设备命令发送时间
设备命令返回时间
SMB100A dwell time
磁场 settle time
laser warmup / settle
OE1022D time constant
采样窗口选择
前段数据丢弃
重复测量平均
样本到 recipe step 的对齐
时间误差模型
```

本模块不追求第一阶段实现硬件级同步，而是先实现一套 **软件同步误差可见、可估计、可回放** 的模型。目标不是声称每个样本绝对同步，而是让每个样本的时间来源、误差来源、所属 step 和使用方式都能被明确记录。

---

## 1. Purpose

本模块解决的问题是：**实验数据不能只记录数值，还必须记录这个数值属于哪个时间窗口、哪个 recipe step、哪个设备状态。**

ODMR 自动化实验中，一个数据点通常不是瞬时值，而是多个过程的结果：

```text
SMB100A 设置 RF / FM / sweep 参数
磁场线圈切换到目标 B vector
激光器进入稳定状态
OE1022D 锁相输出经过时间常数滤波
采集线程持续读取 RALL? frame
Executor 在某个窗口内选取样本
Aggregator 对窗口内样本做平均或统计
```

如果没有统一时间模型，会出现以下问题：

```text
数据点无法对应具体频率或磁场 step
命令发送后还没稳定就开始平均
锁相时间常数导致前段数据污染平均值
SMB dwell 与软件采样窗口不一致
UI 曲线看起来正常但 raw 数据无法解释
重复扫频后无法区分漂移、噪声和真实响应
实验后无法重建每个点的有效采样窗口
机器学习数据库缺少可靠 label
```

因此，本模块的核心目标是：

```text
定义统一的时间戳体系；
定义 step-level timing plan；
定义设备 settle / dwell / pre-discard / averaging window；
定义样本归属规则；
定义软件同步误差模型；
定义重复测量和平均策略；
确保每个 aggregated point 可回溯到 raw sample、step_id、device state 与 timing report。
```

---

## 2. Scope

### 2.1 本模块负责

```text
定义 monotonic timestamp 与 wall timestamp 的使用边界
定义 step timing lifecycle
定义 command send / ack / settle / acquisition / average 的事件模型
定义 dwell、settle、pre_discard、acquisition_window、average_window
定义 OE1022D time constant 与采样窗口的关系
定义重复测量 repeat / averaging / aggregation 的规则
定义 sample-to-step alignment 规则
定义 timing_plan.json 数据结构
定义 timing_report.json 数据结构
定义 timing-related events.jsonl 字段
定义 timing validation 规则
定义 fake clock / jitter / delay / replay 测试要求
```

### 2.2 本模块不负责

```text
不直接控制硬件设备
不实现 OE1022D RALL? parser
不实现 raw bin 文件格式
不定义完整 recipe schema
不定义 SMB100A 所有 SCPI 命令
不定义磁场 coil matrix
不定义 GUI 图表刷新策略
不做复杂物理建模
不做机器学习训练
不声称第一阶段具备硬件级同步
```

### 2.3 与其他 PRD 的边界

| 模块 | 关系 |
|---|---|
| 03 OE1022D Acquisition | 提供 monotonic timestamped raw samples / parsed samples |
| 04 Recipe Schema | 定义 timing 字段在 recipe / resolved_recipe 中的 schema |
| 05 Compiler & Executor | 调用本模块计算 timing plan，并在运行时写入 timing events |
| 07 Data Logging | 保存 timing_plan.json、timing_report.json、events.jsonl、parsed samples |
| 08 GUI | 展示 timing warning、estimated duration、sample window、aggregation status |
| 09 Magnetic Planner | 提供磁场 ramp / settle time 估计 |
| 10 Safety & Interlock | 使用 timing validation 结果拒绝不安全或无效 recipe |
| 11 Harness / Mock / Replay | 提供 fake clock、jitter、delay、timestamp alignment tests |
| 12 Agent Workflow | 限制 AI 只能生成 timing 参数建议，不允许绕过 validation |

---

## 3. Non-goals

第一阶段不做：

```text
纳秒级硬同步
PTP / IEEE 1588 强制部署
多主机分布式同步
外部触发硬件闭环同步
FPGA 级采集调度
SMB100A sweep trigger 与 OE1022D frame 的硬件锁定
在线自适应 dwell 时间优化
AI 根据实时数据动态改采样窗口
复杂 Allan variance 在线计算
复杂贝叶斯采样策略
```

第一阶段只做：

```text
单主机软件时间模型
Rust monotonic clock 作为 duration / ordering 基准
wall timestamp 只作为人类可读日志时间
设备命令生命周期记录
保守的 settle / discard / average 规则
raw samples 到 step window 的后处理对齐
repeat-level 平均和标准差
timestamp alignment replay test
```

---

## 4. Timing Principles

### 4.1 两类时间戳

系统必须同时记录两类时间戳：

| 字段 | 用途 | 是否用于 duration 计算 |
|---|---|---:|
| `monotonic_ns` | 实验内排序、duration、window assignment | 是 |
| `wall_time` | 人类可读日志、文件名、跨实验粗略对照 | 否 |

原则：

```text
所有 duration 计算必须基于 monotonic_ns。
所有 sample-to-step 对齐必须基于 monotonic_ns。
wall_time 不能用于采样窗口计算。
wall_time 允许被系统校时、NTP、人工修改影响。
monotonic_ns 必须从 run_start_monotonic 起算为相对纳秒值。
```

Rust 实现建议使用 `std::time::Instant` 作为 monotonic reference。Rust 官方文档说明 `Instant` 在 tier 1 平台上会尽量使用操作系统提供的单调时钟 API，但仍需注意极少数硬件、虚拟化或操作系统 bug。参考：

```text
https://doc.rust-lang.org/std/time/struct.Instant.html
```

### 4.2 run-level 时间原点

每次 run 必须定义：

```json
{
  "run_id": "2026-xx-xx_odmr_001",
  "run_start_monotonic_ns": 0,
  "run_start_wall_time": "2026-xx-xxTxx:xx:xx.xxxxxx+08:00",
  "clock_model": {
    "monotonic_source": "rust_std_time_instant",
    "wall_source": "system_clock",
    "duration_basis": "monotonic_ns",
    "wall_time_for_duration": false
  }
}
```

所有事件和样本统一记录相对 `run_start_monotonic_ns` 的纳秒值：

```json
{
  "event_name": "step.started",
  "step_id": "step_000123",
  "monotonic_ns": 13500000000,
  "wall_time": "2026-xx-xxTxx:xx:xx.xxxxxx+08:00"
}
```

### 4.3 单主机优先

第一阶段默认：

```text
所有设备控制、采集、执行、落盘在同一台主机中完成。
```

如果未来进入多主机、多设备控制器同步，可以研究 PTP / IEEE 1588。NIST 对 IEEE 1588 的描述是面向网络化测量与控制系统的高精度时钟同步协议。第一阶段不把它作为依赖，只在 PRD 中预留升级路径。参考：

```text
https://www.nist.gov/el/intelligent-systems-division-73500/ieee-1588
```

---

## 5. Runtime Timing Lifecycle

每个执行 step 的时间生命周期定义如下：

```text
step_planned
step_started
command_send
command_ack
settle_started
settle_completed
pre_discard_started
pre_discard_completed
acquisition_window_started
acquisition_window_completed
average_completed
step_completed
```

### 5.1 Step timing diagram

```text
|----------------------------- step dwell / step duration -----------------------------|

step_started
   |
   |-- command_send(device A)
   |-- command_ack(device A)
   |-- command_send(device B)
   |-- command_ack(device B)
   |
   |-- settle_start ------------------------------------------------ settle_end
   |                                                                  |
   |-- pre_discard_start -------------------------------------- pre_discard_end
   |                                                                  |
   |-- acquisition_window_start -------------------------- acquisition_window_end
   |                                                                  |
   |-- aggregation / average
   |
step_completed
```

### 5.2 时间字段定义

| 字段 | 含义 |
|---|---|
| `t_step_start_ns` | Executor 开始执行该 step 的 monotonic time |
| `t_command_send_ns` | 单条设备命令写入 transport 前的时间 |
| `t_command_ack_ns` | 设备响应、ACK、query 验证或超时结束时间 |
| `t_settle_start_ns` | 所有关键设备动作完成后开始等待稳定的时间 |
| `t_settle_end_ns` | 设备稳定等待结束时间 |
| `t_pre_discard_start_ns` | 开始丢弃前段数据的时间 |
| `t_pre_discard_end_ns` | 前段数据丢弃结束时间 |
| `t_acq_window_start_ns` | 有效采样窗口开始时间 |
| `t_acq_window_end_ns` | 有效采样窗口结束时间 |
| `t_step_end_ns` | step 执行结束时间 |

### 5.3 Half-open window 规则

所有采样窗口采用半开区间：

```text
[start_ns, end_ns)
```

样本归属规则：

```text
sample.monotonic_ns >= window.start_ns
sample.monotonic_ns <  window.end_ns
```

禁止使用闭区间 `[start, end]`，避免边界样本被相邻 step 重复计入。

---

## 6. Inputs

### 6.1 来自 resolved_recipe.json

```json
{
  "step_id": "step_000123",
  "device_actions": [
    {"device_id": "smb100a", "action": "set_frequency", "value_hz": 2882000000},
    {"device_id": "mag_z", "action": "set_field", "value_mT": 2.0}
  ],
  "timing": {
    "dwell_ms": 500,
    "settle_ms": 100,
    "pre_discard_ms": 50,
    "acquisition_ms": 300,
    "aggregation": "mean_std"
  }
}
```

### 6.2 来自 device_profile.json

```json
{
  "device_id": "oe1022d",
  "type": "lockin",
  "default_time_constant_ms": 300,
  "default_filter_slope_db_oct": 12,
  "recommended_discard_tau_multiplier": 3,
  "recommended_min_average_tau_multiplier": 2
}
```

```json
{
  "device_id": "smb100a",
  "type": "rf_signal_generator",
  "default_sweep_dwell_ms": 500,
  "frequency_switch_settle_ms": 20,
  "query_after_set": true
}
```

### 6.3 来自 station.json

```json
{
  "station_id": "odmr_station_001",
  "clock_policy": {
    "duration_basis": "monotonic_ns",
    "record_wall_time": true,
    "allow_external_ptp": false
  },
  "timing_policy": {
    "strict_window_validation": true,
    "reject_if_effective_window_too_short": true,
    "default_command_timeout_ms": 2000
  }
}
```

### 6.4 来自采集模块

OE1022D acquisition core 必须提供带 monotonic timestamp 的 sample stream：

```json
{
  "sample_id": 123456,
  "monotonic_ns": 9876543210,
  "frame_seq": 123456,
  "channel": "B",
  "x_vrms": -0.000059173,
  "y_vrms": -0.000040944,
  "r_vrms": 0.000071957,
  "theta_deg": 145.32,
  "freq_hz": 499.999,
  "noise_vrms": 0.000071990,
  "flags": []
}
```

---

## 7. Outputs

### 7.1 timing_plan.json

Compiler 生成 `timing_plan.json`，用于描述预期时间结构。

```json
{
  "schema_version": "0.2",
  "run_id": "planned_run",
  "clock_model": {
    "duration_basis": "monotonic_ns",
    "wall_time_for_duration": false
  },
  "steps": [
    {
      "step_id": "step_000123",
      "planned_dwell_ms": 500,
      "planned_settle_ms": 100,
      "planned_pre_discard_ms": 50,
      "planned_acquisition_ms": 300,
      "planned_margin_ms": 50,
      "aggregation": {
        "mode": "mean_std",
        "min_samples": 5,
        "reject_on_sample_gap": true
      },
      "derived": {
        "effective_average_window_ms": 300,
        "estimated_total_step_ms": 500
      }
    }
  ]
}
```

### 7.2 timing_report.json

Executor 运行后生成 `timing_report.json`。

```json
{
  "schema_version": "0.2",
  "run_id": "2026-xx-xx_odmr_001",
  "summary": {
    "total_steps": 201,
    "completed_steps": 201,
    "timing_warnings": 3,
    "timing_errors": 0,
    "max_command_latency_ms": 42.1,
    "max_sample_gap_ms": 74.0
  },
  "steps": [
    {
      "step_id": "step_000123",
      "t_step_start_ns": 1000000000,
      "t_settle_end_ns": 1120000000,
      "t_acq_window_start_ns": 1170000000,
      "t_acq_window_end_ns": 1470000000,
      "sample_count": 6,
      "effective_average_window_ms": 300.0,
      "timing_status": "ok",
      "warnings": []
    }
  ]
}
```

### 7.3 aggregated_points.parquet / sweep_points.parquet 字段

本模块规定 aggregated point 至少包含：

```text
run_id
step_id
repeat_id
sweep_id
t_acq_window_start_ns
t_acq_window_end_ns
sample_count
sample_first_seq
sample_last_seq
x_mean
x_std
x_min
x_max
y_mean
y_std
r_mean
r_std
theta_mean
freq_mean_hz
noise_mean_vrms
aggregation_mode
timing_status
timing_warning_count
```

---

## 8. Timing Data Model

### 8.1 Step timing object

```json
{
  "step_id": "step_000123",
  "timing_mode": "host_stepped",
  "planned": {
    "dwell_ms": 500,
    "settle_ms": 100,
    "pre_discard_ms": 50,
    "acquisition_ms": 300
  },
  "actual": {
    "t_step_start_ns": 1000000000,
    "t_command_first_send_ns": 1001000000,
    "t_command_last_ack_ns": 1042000000,
    "t_settle_start_ns": 1042000000,
    "t_settle_end_ns": 1142000000,
    "t_pre_discard_start_ns": 1142000000,
    "t_pre_discard_end_ns": 1192000000,
    "t_acq_window_start_ns": 1192000000,
    "t_acq_window_end_ns": 1492000000,
    "t_step_end_ns": 1500000000
  },
  "validation": {
    "status": "ok",
    "warnings": [],
    "errors": []
  }
}
```

### 8.2 Timing modes

| mode | 含义 | v0.2 推荐度 |
|---|---|---:|
| `host_stepped` | Executor 每个 step 显式设置设备参数并记录窗口 | 高 |
| `device_sweep_observed` | 设备内部扫频，软件只记录开始/结束和估算点窗口 | 中低 |
| `external_triggered` | 外部触发同步，软件记录触发事件 | 预留 |
| `hardware_timestamped` | 设备返回硬件 timestamp | 预留 |

第一阶段推荐 `host_stepped`，因为它最容易保证：

```text
每个 step 有 command_send / command_ack
每个 step 有明确 settle / discard / average window
每个 sample 可以明确归属 step_id
```

SMB100A 内部 sweep 可作为兼容模式，但如果没有同步输出或明确触发记录，不能作为第一阶段主采集模式。SMB100A 图8配置中 100 MHz span、500 kHz step、500 ms dwell 对应约 201 个点和约 100.5 s 单次扫频，这类信息可用于 timing estimate，但真实净时间还必须记录软件同步、切换和采集开销。

### 8.3 Device action timing object

```json
{
  "step_id": "step_000123",
  "device_id": "smb100a",
  "action_id": "set_frequency",
  "command": "FREQ 2.882GHz",
  "t_send_ns": 1001000000,
  "t_ack_ns": 1017000000,
  "latency_ms": 16.0,
  "ack_type": "query_verified",
  "status": "ok"
}
```

---

## 9. Timing Validation Rules

### 9.1 基本规则

Compiler 必须检查：

```text
dwell_ms > 0
settle_ms >= 0
pre_discard_ms >= 0
acquisition_ms > 0
dwell_ms >= settle_ms + pre_discard_ms + acquisition_ms + margin_ms
```

默认建议：

```text
margin_ms >= max(20 ms, 2 * expected_sample_period_ms)
```

### 9.2 OE1022D time constant 规则

锁相放大器输出经过低通滤波，刚切换频率、磁场、激光功率或调制状态后，前段输出不能直接平均。

定义：

```text
tau_ms = OE1022D time constant
pre_discard_ms >= discard_tau_multiplier * tau_ms
acquisition_ms >= average_tau_multiplier * tau_ms
```

第一阶段默认：

```text
discard_tau_multiplier = 3
average_tau_multiplier = 2
```

如果用户需要更快扫描，可手动降低 multiplier，但必须在 `timing_report.json` 中记录 warning：

```text
WARNING_TIMING_LOCKIN_WINDOW_SHORTER_THAN_RECOMMENDED
```

### 9.3 磁场 settle 规则

磁场切换后必须等待：

```text
mag_settle_ms = max(profile.default_settle_ms, planner.estimated_settle_ms, recipe.override_settle_ms)
```

如果磁场 step 包含大电流变化或接近 ramp limit，Magnetic Planner 必须给出更保守的 settle estimate。

### 9.4 Laser settle / warmup 规则

激光器相关 timing 分两类：

```text
laser_warmup_ms: 激光开机或从 standby 到 output 的长期稳定时间
laser_settle_ms: 每次功率或状态切换后的短期稳定时间
```

Compiler 必须区分：

```text
run_start_laser_warmup
per_step_laser_settle
```

禁止把 warmup 混入每个 step，否则会严重高估实验时长。

### 9.5 SMB100A dwell 规则

对于 `host_stepped` 模式：

```text
step dwell 由 Executor 控制。
SMB100A 只作为 CW/frequency setpoint。
```

对于 `device_sweep_observed` 模式：

```text
SMB100A dwell 由仪器内部 sweep 控制。
软件必须记录 sweep_start / sweep_stop / estimated_step_windows。
没有外部同步时，timing_status 不能标记为 hard_aligned。
```

SMB100A 的 RF Frequency Sweep 包含 spacing、shape、step、dwell 等参数；对于图8中 `SWE:DWEL 500ms` 这类配置，timing module 可以用它估算点窗口，但仍需记录实际软件事件和延迟。

---

## 10. Sample Window Selection

### 10.1 有效窗口定义

对于每个 step：

```text
effective_window_start_ns = t_settle_end_ns + pre_discard_ns
effective_window_end_ns   = min(t_step_end_ns, t_next_step_start_ns)
```

如果 recipe 显式指定 acquisition window：

```text
effective_window_end_ns = effective_window_start_ns + acquisition_ms
```

但必须满足：

```text
effective_window_end_ns <= t_step_end_ns
```

### 10.2 样本选择

```text
selected_samples = samples where:
  sample.monotonic_ns >= effective_window_start_ns
  sample.monotonic_ns <  effective_window_end_ns
  sample.channel == configured_channel
  sample.flags does not contain fatal flags
```

### 10.3 样本不足处理

如果样本数低于 `min_samples`：

```text
严格模式：step aggregation failed
宽松模式：输出 NaN / null aggregate，并记录 timing warning
```

默认：

```text
strict_window_validation = true
```

### 10.4 sample gap 检查

如果窗口内连续样本间隔超过阈值：

```text
max_gap_ms > max_allowed_sample_gap_ms
```

则记录：

```text
WARNING_TIMING_SAMPLE_GAP_DETECTED
```

如果 recipe 设置 `reject_on_sample_gap = true`，该 step 标记为 failed。

---

## 11. Averaging Model

### 11.1 Aggregation level

系统支持三层平均：

| 层级 | 对象 | 输出 |
|---|---|---|
| sample-level | raw / parsed samples | 不改变 raw，只筛选窗口 |
| step-level | 一个 step 内的样本 | aggregated point |
| repeat-level | 多次重复 step 或 sweep | mean/std/count/drift |

### 11.2 v0.2 必须实现的 aggregation mode

```text
none
mean
mean_std
median
trimmed_mean
```

默认：

```text
mean_std
```

### 11.3 step-level aggregate

对每个 step 输出：

```json
{
  "step_id": "step_000123",
  "aggregation_mode": "mean_std",
  "sample_count": 6,
  "x_mean": -5.91e-5,
  "x_std": 2.3e-7,
  "y_mean": -4.09e-5,
  "r_mean": 7.19e-5,
  "theta_mean": 145.2,
  "freq_mean_hz": 499.999,
  "noise_mean_vrms": 7.2e-5
}
```

### 11.4 repeat-level aggregate

重复扫描时，每个 logical point 可能有多个 repeat：

```text
repeat_id = 0, 1, 2, ...
logical_point_id = frequency + B vector + laser power + other controlled variables
```

输出：

```json
{
  "logical_point_id": "freq_2882000000_Bz_2p0mT",
  "repeat_count": 5,
  "x_repeat_mean": -5.91e-5,
  "x_repeat_std": 3.1e-7,
  "x_repeat_sem": 1.4e-7,
  "drift_estimate": {
    "first_repeat_x": -5.89e-5,
    "last_repeat_x": -5.94e-5,
    "linear_drift_per_repeat": -1.2e-8
  }
}
```

### 11.5 禁止事项

```text
禁止实时路径只保存平均值而丢弃 raw sample
禁止用 CSV 作为实时 aggregation 输入
禁止 GUI 端自行平均 raw frame
禁止 AI agent 修改 averaging 后覆盖 raw data
禁止没有 sample_count / window / timing_status 的 averaged point
```

---

## 12. Software Synchronization Error Model

### 12.1 误差来源

本模块必须显式记录或估计以下误差来源：

| 误差项 | 来源 | 记录方式 |
|---|---|---|
| `command_latency_ms` | host 到 device 的 transport 延迟 | send/ack timestamps |
| `device_apply_delay_ms` | 设备收到命令后内部生效延迟 | profile estimate / query verify |
| `settle_uncertainty_ms` | 磁场、激光、RF、锁相稳定不确定性 | profile + warning |
| `sample_period_jitter_ms` | OE1022D 采集间隔抖动 | sample timestamp gap |
| `parser_queue_delay_ms` | parser / queue 延迟 | acquisition stats |
| `ui_delay_ms` | UI 更新延迟 | 不参与数据窗口 |
| `wall_clock_offset_ms` | 系统时间漂移或校时 | 不参与 duration |

### 12.2 Step timing uncertainty estimate

每个 step 可输出粗略误差估计：

```json
{
  "step_id": "step_000123",
  "timing_uncertainty": {
    "command_latency_ms": 16.0,
    "estimated_device_apply_delay_ms": 20.0,
    "settle_uncertainty_ms": 50.0,
    "max_sample_gap_ms": 52.0,
    "window_boundary_uncertainty_ms": 52.0,
    "quality": "software_aligned"
  }
}
```

### 12.3 Quality label

| label | 含义 |
|---|---|
| `software_aligned` | 软件时间戳对齐，第一阶段默认 |
| `query_verified` | 每步命令经 query 验证后进入 settle |
| `externally_triggered` | 外部触发参与同步，预留 |
| `hardware_timestamped` | 设备硬件 timestamp，预留 |
| `estimated_only` | 只根据 sweep 参数估算，无逐点确认 |

---

## 13. Runtime Model

### 13.1 Compiler phase

Compiler 调用 timing planner：

```text
resolved_recipe.json
+ station timing policy
+ device_profile timing defaults
+ recipe timing overrides
= timing_plan.json
```

Compiler 必须拒绝：

```text
窗口长度不足
dwell 小于 settle + discard + acquisition
min_samples 不可能满足
time constant 与 dwell 明显冲突
device_sweep_observed 缺少 sweep start/stop 策略
```

### 13.2 Executor phase

Executor 按 step 执行：

```text
emit step.started
send device commands
emit command.send / command.ack
wait settle
emit settle.completed
start discard window
start acquisition window
collect or mark samples
aggregate selected samples
emit step.completed
```

### 13.3 Acquisition phase

OE1022D acquisition core 不理解 recipe。它只负责：

```text
持续采集
给每个 frame/sample 打 monotonic timestamp
写 raw bin / parsed stream
报告 frame gap / parser error / queue overflow
```

### 13.4 Aggregator phase

Aggregator 负责：

```text
读取 step windows
读取 parsed samples
按 half-open interval 选择样本
执行 aggregation
输出 aggregated_points
写 timing warnings
```

Aggregator 可以运行在：

```text
near-real-time mode: 为 UI 提供当前 step 的预览值
post-run mode: 实验结束后完整重算 parsed.parquet / aggregated_points.parquet
```

**最终数据以 post-run aggregation 为准。**

---

## 14. API / Interface

### 14.1 Timing planner API

```rust
pub fn build_timing_plan(
    resolved_recipe: &ResolvedRecipe,
    station: &StationConfig,
    profiles: &DeviceProfiles,
) -> Result<TimingPlan, TimingPlanError>;
```

### 14.2 Runtime event API

```rust
pub trait TimingEventSink {
    fn emit_timing_event(&self, event: TimingEvent) -> Result<(), TimingError>;
}
```

### 14.3 Clock API

```rust
pub trait ExperimentClock {
    fn now_monotonic_ns(&self) -> u64;
    fn now_wall_time_rfc3339(&self) -> String;
}
```

必须提供：

```text
SystemClock: 真实运行使用
FakeClock: harness / replay / unit test 使用
```

### 14.4 Aggregation API

```rust
pub fn aggregate_step_window(
    step_window: &StepWindow,
    samples: &[ParsedSample],
    config: &AggregationConfig,
) -> Result<AggregatedPoint, AggregationError>;
```

### 14.5 Timing report API

```rust
pub fn generate_timing_report(
    timing_plan: &TimingPlan,
    events: &[RunEvent],
    sample_stats: &SampleTimingStats,
) -> Result<TimingReport, TimingReportError>;
```

---

## 15. Events

### 15.1 Event naming

事件命名使用 namespace 风格，便于日志过滤和后期 OpenTelemetry 接入。OpenTelemetry 的 semantic conventions 强调统一属性命名有助于跨代码库、库和平台标准化遥测数据；本项目不强依赖 OpenTelemetry，但采用类似命名思想。参考：

```text
https://opentelemetry.io/docs/concepts/semantic-conventions/
```

### 15.2 必须记录的事件

```text
run.started
run.completed
step.started
step.completed
device.command.send
device.command.ack
device.command.timeout
timing.settle.started
timing.settle.completed
timing.discard.started
timing.discard.completed
timing.window.started
timing.window.completed
timing.aggregate.completed
timing.warning
timing.error
```

### 15.3 device.command.send

```json
{
  "event_name": "device.command.send",
  "run_id": "2026-xx-xx_odmr_001",
  "step_id": "step_000123",
  "device_id": "smb100a",
  "action_id": "set_frequency",
  "monotonic_ns": 1001000000,
  "wall_time": "2026-xx-xxTxx:xx:xx.xxxxxx+08:00",
  "payload": {
    "command_family": "scpi",
    "command_redacted": "FREQ <value>",
    "expected_ack": "query_verified"
  }
}
```

### 15.4 timing.window.completed

```json
{
  "event_name": "timing.window.completed",
  "run_id": "2026-xx-xx_odmr_001",
  "step_id": "step_000123",
  "monotonic_ns": 1492000000,
  "payload": {
    "window_type": "acquisition",
    "start_ns": 1192000000,
    "end_ns": 1492000000,
    "duration_ms": 300.0
  }
}
```

### 15.5 timing.aggregate.completed

```json
{
  "event_name": "timing.aggregate.completed",
  "run_id": "2026-xx-xx_odmr_001",
  "step_id": "step_000123",
  "monotonic_ns": 1500000000,
  "payload": {
    "aggregation_mode": "mean_std",
    "sample_count": 6,
    "sample_first_seq": 123450,
    "sample_last_seq": 123455,
    "status": "ok"
  }
}
```

---

## 16. Safety Rules

### 16.1 Timing safety rejection

Compiler 或 Executor 必须拒绝以下情况：

```text
dwell_ms <= 0
acquisition_ms <= 0
settle_ms < 0
pre_discard_ms < 0
dwell_ms < settle_ms + pre_discard_ms + acquisition_ms + margin_ms
OE1022D time constant 未定义且 recipe 要求 auto discard by tau
sample window 无法达到 min_samples
step timing 中出现 end < start
events monotonic order 违反
run 中出现大规模 timestamp gap 且 recipe 要求 strict
```

### 16.2 Runtime safety stop

运行时如果出现以下情况，应停止当前 run 或进入 safe pause：

```text
acquisition thread stopped
sample timestamp 不单调
raw bin writer failed
events writer failed
device command timeout 超过阈值
timing window 连续失败超过阈值
emergency stop 被触发
```

### 16.3 AI 相关安全

AI agent 不能：

```text
关闭 timing validation
把 wall_time 改成 duration basis
强行降低 settle / discard 到 safety policy 以下
删除 timing_report 中的 warning
生成无 min_samples / no timing window 的 recipe
要求 Executor 接受未 resolved recipe
```

AI agent 可以：

```text
建议 dwell / settle / averaging 参数
根据历史 timing_report 建议更合理的采样窗口
生成 timing block JSON 草案
解释 timing warning
生成后处理分析 notebook
```

---

## 17. Error Handling

### 17.1 Command timeout

如果设备命令超时：

```text
记录 device.command.timeout
标记当前 step failed
如果该命令影响危险设备，进入 safe shutdown
如果该命令非危险且 recipe 允许 skip，进入下一 step
默认不允许 skip
```

### 17.2 Sample gap

如果采样窗口内发现 sample gap：

```text
记录 timing.warning
将 step timing_status 标记为 warning 或 failed
post-run aggregation 仍可输出 sample_count 与 gap 信息
```

### 17.3 Monotonic violation

如果发现事件或样本 `monotonic_ns` 倒退：

```text
立即停止 aggregation
记录 timing.error
run 标记为 invalid_timing
保留 raw data，但禁止自动生成 ML-ready dataset
```

### 17.4 Wall clock jump

如果 wall clock 跳变：

```text
记录 timing.warning.wall_clock_jump
不影响 duration / window assignment
run 继续执行
```

### 17.5 Effective window too short

如果实际命令延迟过大，导致有效采集窗口变短：

```text
若 sample_count >= min_samples：记录 warning，继续
若 sample_count < min_samples：step failed
若连续失败超过 threshold：run paused 或 stopped
```

---

## 18. Test Harness

### 18.1 必须提供的 fake components

```text
FakeClock
FakeOE1022D sample stream
FakeSMB100A command latency model
FakeMagAxis settle model
FakeLaser warmup / settle model
FakeExecutor event stream
Raw replay timing source
```

### 18.2 Unit tests

```text
test_monotonic_clock_increases
test_half_open_window_assignment
test_samples_on_boundary_not_duplicated
test_dwell_too_short_rejected
test_tau_based_pre_discard
test_min_samples_validation
test_sample_gap_detection
test_wall_clock_jump_does_not_affect_duration
test_repeat_level_average
test_trimmed_mean_aggregation
```

### 18.3 Integration tests

```text
fake 201-point RF sweep with 500 ms dwell
fake magnetic sweep with variable settle time
fake laser warmup once + per-step settle
fake OE1022D time constant causing pre-discard
raw bin replay reconstructs identical aggregated points
events.jsonl reconstructs timing_report.json
timestamp alignment tests pass under injected jitter
```

### 18.4 Replay tests

Replay 必须验证：

```text
同一 raw bin + events.jsonl + timing_plan.json 多次 replay 输出相同 aggregated_points
样本到 step_id 映射稳定
边界样本不重复计入
丢帧时 warning 可复现
命令延迟导致窗口缩短时 warning 可复现
```

---

## 19. Acceptance Criteria

### 19.1 功能验收

```text
timing_plan.json 可由 resolved_recipe.json 生成
events.jsonl 包含 command / settle / discard / acquisition / aggregate 事件
每个 aggregated point 包含 step_id 和 acquisition window
每个 aggregated point 包含 sample_count / sample_first_seq / sample_last_seq
post-run aggregation 可从 raw / parsed / events 重建
repeat-level averaging 输出 mean / std / count / sem
```

### 19.2 性能验收

```text
timing event 写入不能阻塞 OE1022D 采集线程
near-real-time aggregation 失败不影响 raw 采集
post-run aggregation 可处理 30 min 连续采集数据
201-point sweep replay 后 step assignment 正确
UI 卡顿不影响 timing model
```

### 19.3 正确性验收

```text
所有 duration 均基于 monotonic_ns
wall_time 跳变不改变采样窗口
半开区间边界样本不重复
dwell 不足时 Compiler 拒绝 recipe
sample gap 可被检测并写入 timing_report
raw replay 多次结果一致
```

### 19.4 安全验收

```text
timing validation failed 的 recipe 不能执行
sample timestamp 非单调时 run 标记 invalid_timing
events writer failed 时 run 停止或进入 safe shutdown
Executor 不能绕过 timing_plan 直接执行未检查 step
AI agent 不能关闭 timing validation
```

---

## 20. Implementation Notes

### 20.1 推荐 crate 划分

```text
crates/
  odmr-timing/
    clock.rs
    event.rs
    planner.rs
    window.rs
    aggregate.rs
    report.rs
    validation.rs
    fake_clock.rs
```

### 20.2 推荐依赖方向

```text
odmr-executor -> odmr-timing
odmr-acquisition -> odmr-timing clock types only
odmr-data -> odmr-timing report types
odmr-gui-api -> odmr-timing DTO only
```

禁止：

```text
odmr-timing 依赖 GUI
odmr-timing 直接访问硬件
odmr-timing 依赖 AI/prompt 模块
odmr-acquisition 依赖 recipe compiler
```

### 20.3 Timing validation 与 safety validation 的关系

```text
Timing validation 属于 recipe 可执行性检查。
Safety validation 属于实验安全边界检查。
两者都必须通过，Executor 才能执行。
```

推荐顺序：

```text
schema validation
semantic validation
safety validation
timing validation
dry-run approval
executor run
```

---

## 21. Open Questions

第一阶段允许保留的问题：

```text
OE1022D RALL? 实际帧率和 jitter 需要实测
不同 time constant 下最佳 discard multiplier 需要实验校准
SMB100A host-stepped 频率切换实际延迟需要 benchmark
磁场线圈 settle time 需要按 coil / driver / current step 实测
激光器 warmup / settle 是否需要光功率监控反馈
是否需要后期支持外部 trigger 或 PTP
是否需要 Allan deviation 作为长期漂移指标
```

这些问题不阻塞 v0.2 架构，但必须通过 benchmark report 和实验日志逐步收敛。

---

## 22. Agent Constraints

### 22.1 Timing PRD Agent

输入：

```text
06 Timing / Synchronization / Averaging PRD
03 OE1022D Acquisition PRD
05 Recipe Compiler & Executor PRD
SMB100A sweep timing notes
OE1022D time constant settings
```

输出：

```text
timing_plan schema draft
timing_report schema draft
Rust timing structs
timing validation tests
sample-to-step alignment tests
aggregation tests
```

禁止：

```text
不改 OE1022D parser
不改 SMB100A driver
不改 GUI 图表实现
不改 safety limit
不生成绕过 timing validation 的 executor 代码
```

### 22.2 Coding Agent Review Checklist

任何涉及本模块的 PR 必须检查：

```text
是否所有 duration 使用 monotonic_ns
是否 wall_time 只用于展示或日志
是否有 fake clock 测试
是否有 half-open window 测试
是否有 sample gap 测试
是否有 dwell too short rejection 测试
是否 raw 数据未被 aggregation 覆盖
是否 timing_report 可由 replay 重建
```

---

## 23. v0.2 Summary

本 PRD v0.2 决定：

```text
第一阶段采用单主机软件同步。
所有 duration 和窗口对齐基于 monotonic_ns。
wall_time 只做人类可读记录。
每个 step 必须有 command / settle / discard / acquisition / aggregate 时间模型。
OE1022D time constant 必须影响 pre_discard 和 average window。
SMB100A 内部 sweep 可以估算，但第一阶段优先 host-stepped 模式。
所有 averaged point 必须保留 sample_count、window、step_id、timing_status。
raw sample 永远保留；平均值只是派生结果。
```

---

## 24. References

```text
Rust std::time::Instant documentation
https://doc.rust-lang.org/std/time/struct.Instant.html

NIST IEEE 1588 overview
https://www.nist.gov/el/intelligent-systems-division-73500/ieee-1588

OpenTelemetry Semantic Conventions
https://opentelemetry.io/docs/concepts/semantic-conventions/
```

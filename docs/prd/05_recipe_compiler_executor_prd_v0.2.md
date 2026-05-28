# Sub-PRD 05: Recipe Compiler & Executor PRD v0.2

> 文件定位：Recipe Compiler & Executor Sub-PRD  
> 所属项目：odor-ctl-web / ODMR Automation  
> 上级文档：00_main_prd.md v0.2、01_architecture_prd.md v0.2、02_device_registry_connection_prd.md v0.2、03_oe1022d_acquisition_prd.md v0.2、04_recipe_json_schema_prd.md v0.2  
> 当前阶段：架构冻结前的 v0.2 草案  
> 目标读者：Compiler/Executor 开发者、Safety 开发者、Device Driver 开发者、GUI 开发者、AI/coding agent、实验操作者  

---

## 0. 文档目的

本文件定义 ODMR 自动化系统中的 **Recipe Compiler & Executor** 模块。

该模块负责把通过 schema/semantic/safety 检查的 `recipe.json`、`device_profile.json`、`block.json`、`station.json` 编译为明确、顺序、可审计、可 dry-run、可执行的 `resolved_recipe.json`，并由 Executor 按步骤申请设备资源、发送设备动作、启动/停止采集、写入事件日志、处理 stop/error/emergency。

本文件强调两条边界：

```text
Compiler 不碰真实硬件。
Executor 不接受未 resolved 的 recipe。
```

---

## 1. Purpose

本模块解决的问题是：**实验方案不能直接等于硬件执行流程**。

人类或 AI 生成的 `recipe.json` 表达的是实验意图，例如：

```text
扫 SMB100A RF frequency
在每个频率点采集 OE1022D Ch-B X
对不同 Bz 磁场重复扫描
保存 raw data 与 metadata
```

但真实执行时必须变成明确的设备动作序列：

```text
连接设备
检查 IDN / capability
设置 SMB100A RF/LF/FM
设置 OE1022D input/reference/filter
设置磁场电流
等待 settle
启动或标记采集窗口
记录 step_started
等待 dwell/acquisition window
记录 step_completed
进入下一步
安全停止
```

如果缺少 Compiler & Executor 边界，会出现：

```text
AI 直接生成设备命令
GUI 直接改硬件状态
recipe 没有完全展开
sweep 点和采集数据无法对应
设备状态无法复现
安全检查被绕过
中途停止后 run directory 不完整
```

因此，本模块的核心目标是：

```text
把实验意图编译成确定性执行计划；
把执行计划转换为受控硬件动作；
把每个动作、错误、停止、安全事件写入 events.jsonl；
保证每个数据点能回溯到 step_id、recipe、station_snapshot 和设备状态。
```

---

## 2. Scope

### 2.1 Compiler 负责

Compiler 负责离线或运行前的计划展开：

```text
读取 station.json
读取 device_profile.json
读取 block.json
读取 recipe.json
检查引用关系
展开 block
展开 sweep
计算 sweep point count
计算预计时长
插入 dwell / settle / pre_discard / acquisition window
规范化设备动作
生成 resolved_recipe.json
生成 dry_run_plan.json
生成 compile_report.json
调用或接收 safety_report.json
生成 recipe hash / resolved_recipe hash
```

### 2.2 Executor 负责

Executor 负责运行时执行：

```text
读取 resolved_recipe.json
读取 safety_report.json
确认 safety_report.status == passed
确认 dry-run 已被人工批准
申请设备 resource lease
确认 station/device connection 状态
写入 station_snapshot.json
按 step 顺序执行 device_actions
启动、标记或停止 acquisition core
记录 events.jsonl
处理 pause / stop / emergency stop
处理 timeout / disconnect / device error
执行 safe shutdown
生成 run_summary.json
```

### 2.3 本模块不负责

```text
不定义完整 JSON Schema 细节：由 04 PRD 负责
不实现 OE1022D RALL? 高频采集：由 03 PRD 负责
不实现设备发现和连接配置：由 02 PRD 负责
不实现安全上限定义：由 10 PRD 负责
不实现 raw bin / parquet 格式：由 07 PRD 负责
不实现 GUI 页面：由 08 PRD 负责
不实现磁场 planner UI：由 09 PRD 负责
不实现 AI prompt 和 agent 编排：由 12 PRD 负责
不执行复杂数据分析和论文拟合
```

---

## 3. Non-goals

第一阶段不做：

```text
运行时动态修改 recipe
AI 在线闭环直接修改正在执行的 resolved_recipe
复杂 DAG / workflow engine
多 recipe 并发执行
跨多台电脑分布式执行
远程多人协作审批
断点续跑所有边界情况
硬实时控制系统
纳秒级硬同步
```

第一阶段只做：

```text
单 station
单 run lifecycle
顺序 step executor
明确 stop / error / emergency 处理
可 dry-run
可 replay
可 mock
可审计
可追溯
```

---

## 4. Design Principles

### 4.1 Compiler 是纯函数倾向模块

Compiler 应尽量满足：

```text
same station
same profiles
same blocks
same recipe
same compiler version
→ same resolved_recipe
```

允许变化字段：

```text
generated_at
compile_id
absolute output path
operator note
```

不允许无理由变化：

```text
step order
step_id
sweep_coordinates
expected_state
device_actions
safety decisions
estimated_duration_s
```

### 4.2 Executor 是唯一运行时硬件动作入口

运行期间只有 Executor 可以触发设备动作。

```text
GUI 不能直接发设备命令
AI 不能直接发设备命令
Compiler 不能直接发设备命令
Analysis notebook 不能直接发设备命令
```

设备动作必须经过：

```text
resolved_recipe.step.device_actions
→ executor validation
→ resource lease
→ safety runtime guard
→ device manager
→ device driver
→ event log
```

### 4.3 Dry-run 是执行前强制步骤

Executor 启动前必须存在：

```text
resolved_recipe.json
safety_report.json
dry_run_plan.json
human_approval record
```

dry-run 必须展示所有危险动作：

```text
RF output ON
MOD ON
laser enable
magnetic current ramp
long duration run
large sweep point count
high power / high current near limit
```

### 4.4 resolved_recipe 是执行合同

Executor 不重新解释原始 recipe。Executor 只读取 `resolved_recipe.json`。

禁止 Executor：

```text
展开 sweep
解析 block reference
从 AI prompt 生成 step
从 GUI 表单直接构造硬件命令
接受未通过 safety 的计划
接受人工手改但未重新 hash 的 resolved_recipe
```

### 4.5 事件日志优先于界面状态

运行时发生的一切关键动作都必须进入 `events.jsonl`。

GUI 只是事件和状态的消费者，不是事实来源。

---

## 5. Inputs

### 5.1 Compiler Inputs

| 输入 | 说明 |
|---|---|
| `station.json` | 当前实验站设备、连接、capability、安全 profile 引用 |
| `device_profiles/*.json` | 设备常用配置，例如 SMB100A FM、OE1022D Ch-B 配置 |
| `blocks/*.json` | 可复用实验片段，例如 RF setup、magnetic sweep |
| `recipe.json` | 人类或 AI 生成的实验意图 |
| `schemas/*.schema.json` | JSON Schema 文件 |
| `safety_limits.json` | 安全上限；recipe 不可覆盖 |
| `calibration_snapshot.json` | coil matrix、zero offset、校准信息 |
| `compiler_config.json` | step_id 规则、最大 step 数、最大预计时长等 |

### 5.2 Executor Inputs

| 输入 | 说明 |
|---|---|
| `resolved_recipe.json` | Compiler 输出的最终执行计划 |
| `safety_report.json` | Safety 检查报告，必须 passed |
| `dry_run_plan.json` | 人类可读预览 |
| `human_approval.json` | GUI 或人工确认记录 |
| `station_snapshot.json` | 执行前由 Executor 写入或确认 |
| `device_registry_state` | Device Manager 当前连接状态 |
| `run_config.json` | run_id、输出目录、日志策略 |

### 5.3 Runtime Commands

Executor 接收的运行时控制命令仅限：

```text
start_run
request_stop
request_emergency_stop
request_pause  // v0.2 可定义但可不实现
request_resume // v0.2 可定义但可不实现
query_run_status
query_step_status
```

禁止运行时命令：

```text
modify_step
insert_step
skip_safety
direct_scpi
raw_serial_write
change_safety_limit
change_recipe_during_run
```

---

## 6. Outputs

### 6.1 Compiler Outputs

| 输出 | 说明 |
|---|---|
| `resolved_recipe.json` | 最终执行计划；Executor 唯一接受的计划 |
| `dry_run_plan.json` | GUI 展示用执行预览 |
| `compile_report.json` | 编译过程、展开统计、warning、hash |
| `safety_input.json` | 传给 Safety 模块的规范化参数集合 |
| `safety_report.json` | 可由 Compiler 调用 Safety 生成，也可由 Safety 独立生成 |

### 6.2 Executor Outputs

| 输出 | 说明 |
|---|---|
| `events.jsonl` | 运行事件主日志 |
| `station_snapshot.json` | run 开始时设备、profile、capability、连接状态快照 |
| `resolved_recipe.lock.json` | run 目录中的冻结版 resolved recipe |
| `safety_report.lock.json` | run 目录中的冻结版 safety report |
| `run_summary.json` | run 结束摘要 |
| `error_report.json` | run failed 或 emergency stop 时生成 |
| `device_final_state.json` | run 结束或中断后的设备状态 |

### 6.3 Acquisition-Related Outputs

Executor 不直接写 raw frame，但必须和 Acquisition Core 协调：

```text
acquisition_started event
acquisition_window_opened event
acquisition_window_closed event
step_id marker
raw bin writer status
index.jsonl step range
acquisition_stopped event
```

数据文件由 Data Logging 与 Acquisition Core 负责。

---

## 7. Runtime Model

## 7.1 Compile Pipeline

```text
recipe.json
  → schema validation
  → semantic validation
  → reference resolution
  → block expansion
  → sweep expansion
  → timing insertion
  → device action normalization
  → expected_state generation
  → duration estimation
  → safety input generation
  → safety validation
  → resolved_recipe.json
  → dry_run_plan.json
```

### 7.1.1 Pipeline Gates

| Gate | 失败后行为 |
|---|---|
| Schema validation | 停止；返回字段级错误 |
| Semantic validation | 停止；返回引用/语义错误 |
| Block expansion | 停止；返回 block conflict |
| Sweep expansion | 停止；返回点数/方向/范围错误 |
| Timing insertion | 停止；返回 dwell/settle/window 冲突 |
| Safety validation | 生成 failed safety_report；不允许执行 |
| Dry-run generation | 停止；不允许执行 |

## 7.2 Executor State Machine

```text
Idle
  → Loading
  → Preflight
  → LeasingResources
  → Snapshotting
  → StartingAcquisition
  → Running
  → Stopping
  → SafeShutdown
  → Completed
```

异常路径：

```text
Loading / Preflight / Running
  → Failed
  → SafeShutdown
  → Finalized
```

紧急路径：

```text
Any State
  → EmergencyStopping
  → ForceSafeOutputs
  → Finalized
```

### 7.2.1 State Definitions

| State | 含义 |
|---|---|
| `Idle` | 无 run 正在执行 |
| `Loading` | 读取 resolved_recipe、safety_report、approval |
| `Preflight` | 检查 hash、schema、safety、设备状态 |
| `LeasingResources` | 申请设备资源租约 |
| `Snapshotting` | 写入 station_snapshot 与 lock 文件 |
| `StartingAcquisition` | 启动或准备 acquisition core |
| `Running` | 按 step 执行 |
| `Stopping` | 普通停止请求，完成安全收尾 |
| `EmergencyStopping` | 立即执行危险输出关闭 |
| `SafeShutdown` | 关闭 RF、MOD、laser、磁场 ramp 到安全状态 |
| `Completed` | 正常完成 |
| `Failed` | 非正常失败 |
| `Finalized` | 日志、summary、final state 已写完 |

## 7.3 Step Execution Model

每个 step 的执行顺序：

```text
1. emit step_started
2. verify resource lease still valid
3. runtime safety guard check
4. apply device_actions in deterministic order
5. verify command completion or timeout
6. update expected device state cache
7. wait settle_ms if defined
8. open acquisition window if enabled
9. discard pre_discard_ms if defined
10. collect or mark acquisition window_ms
11. close acquisition window
12. wait remaining dwell_ms if defined
13. emit step_completed
```

### 7.3.1 Device Action Ordering

同一个 step 内设备动作必须有稳定顺序：

```text
safe outputs off if needed
magnetic field ramp
laser setup / enable
SMB RF/LF/FM setup
OE1022D configuration
acquisition marker
RF/MOD output enable if required
```

原则：

```text
先配置，后打开输出
先关闭危险输出，再改危险参数
磁场变化后必须 settle
OE1022D time constant 改变后必须等待足够稳定时间
```

## 7.4 Thread / Task Model

### 7.4.1 Recommended Rust Tasks

```text
executor_task
  持有 run lifecycle，顺序执行 step。

device_command_task
  接收 executor 的设备动作，通过 Device Manager 串行化发送。

event_writer_task
  负责 events.jsonl 顺序写入和 flush。

acquisition_control_task
  负责启动/停止 acquisition core，写 step marker。

status_broadcast_task
  向 GUI 发布降频状态，不阻塞 executor。
```

### 7.4.2 禁止线程关系

```text
GUI thread 不得直接调用 device driver
AI task 不得调用 executor start_run 以外的硬件动作
acquisition thread 不得等待 GUI
executor_task 不得解析 raw frame
device_command_task 不得写 raw bin
```

---

## 8. Data Model

## 8.1 ResolvedRecipe

最小结构：

```json
{
  "schema_version": "0.2.0",
  "kind": "resolved_recipe",
  "id": "resolved_odmr_basic_20260528_001",
  "source_recipe_id": "odmr_basic_frequency_sweep",
  "source_recipe_hash": "sha256:...",
  "compiler_version": "0.2.0",
  "station_id": "station_nv_lab_01",
  "station_hash": "sha256:...",
  "safety_report_id": "safety_report_20260528_001",
  "estimated_duration_s": 402.5,
  "step_count": 201,
  "steps": []
}
```

Required:

```text
schema_version
kind
id
source_recipe_id
source_recipe_hash
compiler_version
station_id
safety_report_id
estimated_duration_s
step_count
steps
```

## 8.2 ResolvedStep

最小结构：

```json
{
  "step_id": "step_000001",
  "phase": "measure",
  "source_block_id": "block_smb_fm_500hz_4mhz",
  "sweep_coordinates": {
    "smb100a.rf.frequency_hz": 2820000000
  },
  "device_actions": [
    {
      "action_id": "act_000001_001",
      "device_id": "smb100a_01",
      "device_type": "smb100a",
      "action": "set_rf_frequency",
      "params": {
        "frequency_hz": 2820000000
      },
      "timeout_ms": 2000,
      "requires_output_off": false,
      "hazard_level": "low"
    }
  ],
  "expected_state": {
    "smb100a_01": {
      "rf_frequency_hz": 2820000000,
      "rf_power_dbm": -15.0
    }
  },
  "timing": {
    "settle_ms": 500,
    "dwell_ms": 500
  },
  "acquisition": {
    "enabled": true,
    "device_id": "oe1022d_01",
    "readout": ["x", "y", "r", "theta", "freq", "noise"],
    "pre_discard_ms": 300,
    "window_ms": 500,
    "step_marker_required": true
  }
}
```

## 8.3 DeviceAction

DeviceAction 是 Executor 与 Device Manager 的合同。

```json
{
  "action_id": "act_000001_001",
  "device_id": "smb100a_01",
  "device_type": "smb100a",
  "action": "set_fm_deviation",
  "params": {
    "deviation_hz": 4000000
  },
  "timeout_ms": 2000,
  "retry": {
    "max_attempts": 1,
    "backoff_ms": 100
  },
  "hazard_level": "medium",
  "dry_run_visible": true
}
```

字段规则：

```text
action_id 必须在 run 内唯一
device_id 必须来自 station.devices
device_type 必须和 registry 匹配
action 必须来自 driver capability
timeout_ms 必须显式定义或由 Compiler 插入默认值
hazard_level 必须由 Compiler/Safety 标注
危险动作 dry_run_visible 必须为 true
```

## 8.4 DryRunPlan

`dry_run_plan.json` 面向人类和 GUI。

```json
{
  "schema_version": "0.2.0",
  "kind": "dry_run_plan",
  "id": "dry_run_20260528_001",
  "resolved_recipe_id": "resolved_odmr_basic_20260528_001",
  "summary": {
    "step_count": 201,
    "estimated_duration_s": 402.5,
    "devices": ["smb100a_01", "oe1022d_01", "mag_xyz_01"],
    "hazard_actions": 3
  },
  "hazards": [
    {
      "step_id": "step_000001",
      "device_id": "smb100a_01",
      "action": "set_rf_output_enabled",
      "params": {
        "enabled": true
      },
      "reason": "RF output will be enabled."
    }
  ],
  "sweep_preview": {
    "axes": ["smb100a.rf.frequency_hz"],
    "first_points": [2820000000, 2820500000],
    "last_points": [2919500000, 2920000000]
  }
}
```

## 8.5 CompileReport

```json
{
  "schema_version": "0.2.0",
  "kind": "compile_report",
  "id": "compile_report_20260528_001",
  "compiler_version": "0.2.0",
  "status": "passed",
  "source_recipe_id": "odmr_basic_frequency_sweep",
  "diagnostics": [],
  "expanded": {
    "block_count": 3,
    "sweep_count": 1,
    "step_count": 201
  },
  "estimated_duration_s": 402.5,
  "hashes": {
    "source_recipe_hash": "sha256:...",
    "resolved_recipe_hash": "sha256:..."
  }
}
```

## 8.6 RunSummary

```json
{
  "schema_version": "0.2.0",
  "kind": "run_summary",
  "run_id": "run_20260528_001",
  "resolved_recipe_id": "resolved_odmr_basic_20260528_001",
  "status": "completed",
  "started_at": "2026-05-28T12:00:00Z",
  "ended_at": "2026-05-28T12:07:00Z",
  "steps_total": 201,
  "steps_completed": 201,
  "steps_failed": 0,
  "frames_recorded": 8040,
  "frames_dropped": 0,
  "safe_shutdown_completed": true
}
```

---

## 9. API / Interface

## 9.1 Compiler API

Rust interface draft:

```rust
pub struct CompileInput {
    pub station: Station,
    pub profiles: Vec<DeviceProfile>,
    pub blocks: Vec<Block>,
    pub recipe: Recipe,
    pub safety_limits: SafetyLimits,
    pub calibration: Option<CalibrationSnapshot>,
    pub config: CompilerConfig,
}

pub struct CompileOutput {
    pub resolved_recipe: ResolvedRecipe,
    pub dry_run_plan: DryRunPlan,
    pub compile_report: CompileReport,
    pub safety_report: SafetyReport,
}

pub trait RecipeCompiler {
    fn compile(&self, input: CompileInput) -> Result<CompileOutput, CompileError>;
}
```

### 9.1.1 Compiler Functions

```text
load_inputs(paths) -> CompileInput
validate_input(input) -> ValidationResult
resolve_references(input) -> ReferenceGraph
expand_blocks(graph) -> ExpandedRecipe
expand_sweeps(expanded) -> Vec<ResolvedStepDraft>
insert_timing(step_drafts) -> Vec<ResolvedStepDraft>
normalize_actions(step_drafts) -> Vec<ResolvedStep>
estimate_duration(steps) -> DurationEstimate
generate_safety_input(steps) -> SafetyInput
generate_dry_run(resolved, safety_report) -> DryRunPlan
write_outputs(output_dir, compile_output) -> WriteResult
```

## 9.2 Executor API

Rust interface draft:

```rust
pub struct RunStartRequest {
    pub run_id: String,
    pub resolved_recipe_path: PathBuf,
    pub safety_report_path: PathBuf,
    pub dry_run_approval_path: PathBuf,
    pub output_dir: PathBuf,
}

pub enum RunControlCommand {
    Start(RunStartRequest),
    RequestStop { run_id: String, reason: String },
    EmergencyStop { run_id: String, reason: String },
    QueryStatus { run_id: String },
}

pub trait RecipeExecutor {
    async fn start_run(&self, req: RunStartRequest) -> Result<RunHandle, ExecutorError>;
    async fn request_stop(&self, run_id: &str, reason: &str) -> Result<(), ExecutorError>;
    async fn emergency_stop(&self, run_id: &str, reason: &str) -> Result<(), ExecutorError>;
    async fn query_status(&self, run_id: &str) -> Result<RunStatus, ExecutorError>;
}
```

## 9.3 Device Manager Interface

Executor 只通过 Device Manager 访问设备。

```rust
pub trait DeviceManager {
    async fn acquire_lease(&self, devices: &[DeviceId], run_id: &str) -> Result<ResourceLease, LeaseError>;
    async fn execute_action(&self, lease: &ResourceLease, action: &DeviceAction) -> Result<DeviceActionResult, DeviceError>;
    async fn query_snapshot(&self, devices: &[DeviceId]) -> Result<DeviceSnapshot, DeviceError>;
    async fn safe_shutdown(&self, lease: &ResourceLease, policy: SafeShutdownPolicy) -> Result<(), DeviceError>;
}
```

## 9.4 Acquisition Core Interface

Executor 不能解析 RALL?，只能控制采集生命周期和 step marker。

```rust
pub trait AcquisitionController {
    async fn start(&self, config: AcquisitionRunConfig) -> Result<(), AcquisitionError>;
    async fn mark_step_start(&self, run_id: &str, step_id: &str) -> Result<(), AcquisitionError>;
    async fn open_window(&self, run_id: &str, step_id: &str) -> Result<(), AcquisitionError>;
    async fn close_window(&self, run_id: &str, step_id: &str) -> Result<(), AcquisitionError>;
    async fn stop(&self, run_id: &str) -> Result<(), AcquisitionError>;
}
```

## 9.5 GUI Interface

GUI 允许：

```text
compile_recipe
show_compile_report
show_dry_run_plan
show_safety_report
approve_dry_run
start_run
request_stop
request_emergency_stop
query_status
show_events
```

GUI 禁止：

```text
直接编辑 resolved_recipe 后启动
直接修改 safety_report
直接调用 device driver
直接发送 SCPI / serial command
直接控制 acquisition thread
```

## 9.6 AI Agent Interface

AI agent 允许：

```text
生成 recipe 草案
生成 block 草案
解释 compile error
解释 safety rejection
建议缩小 sweep 范围
建议增加 dwell / settle / repeat
```

AI agent 禁止：

```text
直接调用 executor.start_run
直接调用 emergency stop 以外的硬件动作
修改 safety limit
要求绕过 dry-run
生成 direct_scpi device action
生成 resolved_recipe 并跳过 compiler
```

---

## 10. Safety Rules

## 10.1 Execution Gates

Executor 启动前必须全部满足：

```text
resolved_recipe.json exists
resolved_recipe schema valid
resolved_recipe hash matches compile_report
safety_report.json exists
safety_report.status == passed
dry_run_plan.json exists
human_approval exists
human_approval references same resolved_recipe_hash
station required devices connected
resource leases acquired
run output directory created
emergency stop path available
```

任一条件失败：

```text
Executor must reject start_run.
```

## 10.2 Runtime Safety Guard

即使已经通过 compile-time safety，Executor 每步执行前仍需做轻量 runtime guard：

```text
device still connected
resource lease still valid
action device_id matches lease
action hazard_level allowed
action params not outside safety_report envelope
emergency stop not active
manual override not active unless allowed
```

## 10.3 Dangerous Actions

危险动作必须 dry-run 可见，并写入 event：

```text
SMB RF output ON
SMB MOD ON
SMB power change near limit
SMB frequency sweep start
laser enable
laser power set
mag current ramp
mag output enable
long acquisition start
manual override
safe shutdown failure
```

## 10.4 Safe Shutdown Order

默认安全关闭顺序：

```text
1. Stop accepting new step execution
2. Close acquisition window marker
3. Stop or flush acquisition core
4. SMB100A RF output OFF
5. SMB100A MOD OFF
6. Laser output OFF or safe standby
7. Magnetic output ramp to safe current according to policy
8. Query final device state if possible
9. Release resource lease
10. Write run_summary / error_report
```

注：磁场 safe shutdown 是否立即归零取决于设备与样品安全策略。若快速归零会造成风险，应按 `safe_shutdown_policy` ramp down。

## 10.5 Safety Limits Are Immutable During Run

运行期间禁止：

```text
修改 smb_max_power_dbm
修改 laser_max_power_mw
修改 mag_max_current_a
修改 mag_max_ramp_rate_a_per_s
修改 emergency stop policy
修改 safe shutdown policy
```

---

## 11. Error Handling

## 11.1 CompileError

错误分类：

| 类型 | 示例 | 行为 |
|---|---|---|
| `SchemaError` | 字段类型错误 | 停止编译 |
| `ReferenceError` | block_id 不存在 | 停止编译 |
| `SweepError` | step 方向错误、点数过大 | 停止编译 |
| `TimingError` | acquisition window > dwell | 停止编译 |
| `ConflictError` | 同设备互斥参数冲突 | 停止编译 |
| `SafetyError` | 超出安全限制 | 生成 failed safety_report，禁止执行 |
| `InternalError` | compiler bug | 停止并要求开发修复 |

CompileError 必须结构化：

```json
{
  "error_type": "SweepError",
  "message": "Sweep step sign does not move from start to stop.",
  "path": "$.sweeps[0].step",
  "severity": "error",
  "suggestion": "Use a positive step for ascending sweep."
}
```

## 11.2 ExecutorError

错误分类：

| 类型 | 示例 | 行为 |
|---|---|---|
| `PreflightError` | safety_report failed | 拒绝启动 |
| `LeaseError` | 设备被占用 | 拒绝启动 |
| `DeviceTimeout` | SMB100A command timeout | retry 或 fail |
| `DeviceDisconnected` | OE1022D 断连 | stop + safe shutdown |
| `AcquisitionError` | raw writer failed | stop + safe shutdown |
| `SafetyViolation` | runtime guard failed | emergency stop |
| `UserStop` | 人工停止 | graceful stop |
| `EmergencyStop` | 急停 | immediate safe outputs |
| `EventWriterError` | events.jsonl 写失败 | 尽可能停止，标记日志异常 |

## 11.3 Retry Policy

默认策略：

```text
危险输出类动作：不自动重试，失败则 stop/emergency
普通查询动作：可重试 1-3 次
普通设置动作：可重试 1 次，重试前查询状态
采集写盘错误：不重试采集本身，立即停止 run
```

## 11.4 Stop Semantics

### 11.4.1 User Stop

普通停止：

```text
不再开始新 step
当前危险动作完成或安全中断
关闭 acquisition window
执行 safe shutdown
run status = stopped
```

### 11.4.2 Emergency Stop

急停：

```text
立即标记 emergency_stop
优先关闭 RF / MOD / laser
磁场按 emergency policy 处理
停止采集或 flush 当前 buffer
释放资源
run status = emergency_stopped
```

### 11.4.3 Failure Stop

错误停止：

```text
记录 error event
根据错误类型判断 graceful stop 或 emergency stop
执行 safe shutdown
生成 error_report.json
run status = failed
```

---

## 12. Event Logging

## 12.1 Event Requirements

Executor 必须写入：

```text
run_created
run_preflight_started
run_preflight_passed
resource_lease_acquired
station_snapshot_written
acquisition_started
run_started
step_started
device_action_started
device_action_completed
device_action_failed
acquisition_window_opened
acquisition_window_closed
step_completed
step_failed
user_stop_requested
emergency_stop_requested
safe_shutdown_started
safe_shutdown_completed
resource_lease_released
run_completed
run_stopped
run_failed
```

## 12.2 Event Structure

```json
{
  "schema_version": "0.2.0",
  "kind": "run_event",
  "run_id": "run_20260528_001",
  "event_id": "evt_000001",
  "timestamp_wall": "2026-05-28T12:00:00Z",
  "timestamp_monotonic_ns": 123456789000,
  "level": "info",
  "event_type": "device_action_completed",
  "step_id": "step_000001",
  "action_id": "act_000001_001",
  "device_id": "smb100a_01",
  "message": "SMB100A RF frequency set.",
  "data": {
    "frequency_hz": 2820000000,
    "duration_ms": 12
  }
}
```

## 12.3 Flush Policy

```text
run lifecycle event: immediate flush
safety event: immediate flush
error event: immediate flush
device action event: batched but bounded flush
step event: flush at step_completed
```

如果 event writer 失败：

```text
Executor must stop run because traceability is broken.
```

---

## 13. Test Harness

## 13.1 Compiler Tests

必须覆盖：

```text
schema valid recipe compile success
missing profile compile failure
missing block compile failure
invalid sweep step rejection
nested sweep expansion order
block parameter expansion
expected_state generation
step_id deterministic generation
duration estimation
safety input generation
dry_run hazard extraction
resolved_recipe snapshot stability
```

## 13.2 Executor Mock Tests

必须覆盖：

```text
fake SMB100A SCPI server
fake OE1022D acquisition controller
fake laser serial
fake mag axis
resource lease success/failure
safe shutdown success/failure
device timeout
user stop
emergency stop
acquisition writer failure
event writer failure
```

## 13.3 Replay Tests

必须覆盖：

```text
resolved_recipe replay without hardware
run events replay
raw bin + index step alignment replay
failed run replay
emergency stop replay
```

## 13.4 Integration Tests

有设备时必须覆盖：

```text
connect station
compile odmr_basic_sweep
approve dry-run
execute short 3-point RF sweep with RF output disabled
execute mock acquisition markers
write complete run directory
safe shutdown
```

真实危险输出测试必须单独 gated：

```text
requires manual flag
requires lab safety approval
requires low-power limit
requires visible dry-run
```

## 13.5 Benchmark Tests

Compiler benchmark：

```text
1 sweep axis, 201 steps
2 nested sweep axes, 201 x 21 steps
large recipe rejection threshold
compile time report
memory usage report
```

Executor benchmark：

```text
mock 1000 step run
event writer throughput
status broadcast does not block execution
stop latency
emergency stop latency
```

---

## 14. Acceptance Criteria

## 14.1 v0.2 PRD Acceptance

本 PRD 完成后应能回答：

```text
Compiler 输入什么，输出什么
Executor 输入什么，输出什么
Compiler 和 Executor 的边界在哪里
为什么 Executor 不接受 recipe.json
什么条件下 Executor 拒绝启动
step 如何执行
事件如何记录
stop / error / emergency 如何处理
如何 mock / replay / test
AI agent 不能碰哪里
```

## 14.2 v0.3 Implementation Acceptance

第一版实现应满足：

```text
能读取 station/profile/block/recipe
能编译 odmr_basic_sweep.recipe.json
能展开 RF frequency sweep
能生成 resolved_recipe.json
能生成 dry_run_plan.json
能生成 compile_report.json
能生成 passed/failed safety_report
Executor 能执行 fake device run
Executor 能写完整 events.jsonl
Executor 能处理中途 user stop
Executor 能处理 fake emergency stop
Executor 能执行 safe shutdown
run directory 可被 replay 工具读取
```

## 14.3 v1.0 Lab Acceptance

实验室可用前必须满足：

```text
短 ODMR sweep 可在真实设备上完成
RF output ON 的动作 dry-run 可见
OE1022D acquisition marker 与 step_id 对齐
连续运行过程中 GUI 卡顿不影响 Executor
普通 stop 能安全关闭 RF / MOD / laser
emergency stop 能进入 safe shutdown
所有数据点能回溯到 resolved_recipe step
```

---

## 15. Agent Constraints

## 15.1 Compiler Agent

允许：

```text
实现 compiler crate
实现 sweep expansion
实现 block expansion
实现 dry-run generator
实现 compile tests
修改 05 PRD 相关代码
```

禁止：

```text
访问真实硬件
修改 device driver
修改 GUI
修改 safety limit
实现 AI prompt
绕过 schema validation
```

## 15.2 Executor Agent

允许：

```text
实现 executor state machine
实现 resource lease 调用
实现 event writer
实现 stop/emergency logic
实现 fake device integration tests
```

禁止：

```text
解析 raw RALL? binary frame
修改 recipe schema
修改 GUI chart
直接接受 AI 生成命令
跳过 safety_report
```

## 15.3 GUI Agent

允许：

```text
展示 compile_report
display dry_run_plan
display safety_report
触发 start_run/request_stop/emergency_stop
显示 run status/events
```

禁止：

```text
直接改 resolved_recipe 后执行
直接向设备发命令
直接读写 raw bin
在前端展开 sweep 作为执行来源
```

## 15.4 AI Recipe Agent

允许：

```text
生成 recipe 草案
根据 compile error 修复 recipe 草案
解释 dry-run
建议参数缩小或增加 dwell
```

禁止：

```text
调用真实硬件
调用 executor.start_run
修改 safety limit
要求跳过 dry-run
生成 direct_scpi 动作
生成 resolved_recipe 作为最终执行输入
```

---

## 16. Open Questions

需要后续 PRD 决定：

```text
pause/resume 是否进入第一阶段实现
Executor 是否支持 step-level retry
磁场 ramp 是否在 Executor 还是 Magnetic Core 内部展开
OE1022D acquisition window 是由 Executor marker 控制还是 Acquisition Core 根据 step schedule 自动控制
事件日志是否采用 OpenTelemetry 兼容字段命名
大规模数据库阶段是否需要 run manifest 与 dataset manifest
```

---

## 17. References

- JSON Schema Draft 2020-12: https://json-schema.org/draft/2020-12
- OpenTelemetry Semantic Conventions: https://opentelemetry.io/docs/concepts/semantic-conventions/


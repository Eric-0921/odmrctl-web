# Sub-PRD 10: Safety & Interlock

Version: v0.2  
Status: Draft for architecture freeze  
Target file: `docs/prd/10_safety_interlock_prd.md`  
Related PRDs: `00_main_prd`, `01_architecture_prd`, `02_device_registry_connection_prd`, `03_oe1022d_acquisition_prd`, `04_recipe_json_schema_prd`, `05_recipe_compiler_executor_prd`, `06_timing_sync_averaging_prd`, `07_data_logging_file_format_prd`, `09_magnetic_field_planner_prd`, `11_harness_mock_replay_prd`, `12_agent_workflow_prd`

---

## 1. Purpose

Safety & Interlock 模块负责定义 ODMR 自动化系统中所有实验安全边界、运行前安全校验、运行中 interlock、危险命令防护、人工急停、故障停机和安全日志。

本模块解决的问题是：

```text
AI / GUI / recipe / executor 都可能提出设备动作
    ↓
设备动作可能打开激光、打开微波、拉高磁场电流、改变锁相输入条件
    ↓
所有危险动作必须先经过统一安全门
    ↓
安全失败时不能执行
    ↓
任何 stop / error / emergency 都必须进入可验证的 safe shutdown
```

本模块的核心原则：

```text
AI 不能修改 safety limit
recipe 不能覆盖 safety limit
GUI 不能绕过 safety gate 直接访问硬件
executor 不能执行未通过安全检查的步骤
所有危险命令必须在 dry-run 中可见
所有安全拒绝、人工 override、emergency stop 必须记录到 events.jsonl
```

---

## 2. Safety Philosophy

### 2.1 Fail-closed

系统默认状态必须是安全关闭状态：

```text
laser output disabled / shutter closed
SMB100A RF output off
SMB100A modulation output off unless explicitly armed
magnetic coil current at zero or configured safe hold current
OE1022D acquisition stopped or in passive read-only mode
executor not armed
```

当系统无法确认设备状态时，必须视为不安全。

```text
unknown state = unsafe state
stale status = unsafe state
missing safety profile = unsafe state
failed device query = unsafe state
```

### 2.2 Layered limits

安全限制分为四层：

```text
hard safety limit       绝对上限，不能被 recipe / GUI / AI 覆盖
station safety limit    当前实验台限制，由 human maintainer 维护
device profile limit    单设备配置限制，必须小于等于 station limit
recipe requested limit  实验方案请求值，必须小于等于 device profile limit
```

优先级：

```text
hard safety limit > station safety limit > device profile limit > recipe value
```

### 2.3 Two-phase execution

所有危险动作必须分两步：

```text
1. Plan / dry-run / safety report
2. Arm / execute
```

不得从 GUI 或 agent 直接触发危险硬件动作。

### 2.4 Human responsibility

本软件提供工程安全层，不替代实验室物理安全规范。

必须由实验人员另行确认：

```text
激光防护眼镜
光路遮挡
微波天线 / 样品空间隔离
线圈散热
电源限流
接地
设备铭牌和手册额定值
```

---

## 3. Scope

### 3.1 本模块负责

```text
laser max power
laser enable / shutter safety
SMB max RF power
SMB allowed frequency range
SMB RF output gating
SMB modulation gating
mag max current
mag max ramp rate
mag safe zero / safe hold
mag settle safety
OE1022D overload handling
emergency stop
safe shutdown
resource lease
manual override rules
safety report
safety event logging
safety rejection tests
runtime interlock checks
```

### 3.2 本模块不负责

```text
激光器底层串口驱动实现
SMB100A SCPI command mapping 细节
磁场 coil matrix 计算细节
OE1022D RALL? parser
recipe sweep 展开算法
GUI 页面具体视觉设计
AI prompt 工程
论文数据分析
```

这些内容分别由对应 PRD 负责。

---

## 4. Hazard Model

### 4.1 Laser hazard

风险：

```text
激光功率过高
激光未预期打开
shutter 状态未知
warmup / settle 状态被误判
recipe 请求超过允许功率
AI 生成错误功率单位
断连后无法确认 laser output 状态
```

安全策略：

```text
laser 默认 off
需要 human arm 才允许打开
打开前必须确认 target power <= max_power_mw
功率单位统一转为 W 内部表示
所有 shutter open / laser enable / set power 属于 dangerous command
laser disconnect 时触发 emergency-safe path
```

### 4.2 RF / microwave hazard

风险：

```text
SMB100A RF output 意外打开
RF power 超限
frequency range 错误
FM deviation 过大
sweep start / stop / dwell 错误
RF output on 时修改关键参数导致瞬态输出
MOD state 与 FM state 混淆
```

安全策略：

```text
RF 默认 off
先设置 frequency / power / modulation，再允许 RF on
RF on 前必须通过 safety gate
power 单位统一为 dBm
frequency 单位统一为 Hz
所有 OUTP ON / power increase / sweep enable 属于 dangerous command
如果 query state 失败，视为 unsafe
```

### 4.3 Magnetic field / coil hazard

风险：

```text
coil current 超限
ramp rate 超限
方向符号错误
zero offset 错误
settle time 不足
长时间大电流导致发热
三轴同时动作导致合成电流或场强超限
mag power supply 断连后保持上一次输出
```

安全策略：

```text
每轴 current limit
每轴 ramp rate limit
合成 current / B field limit
ramp path 必须可 dry-run 展示
zero / safe hold 命令必须优先级最高
executor stop 时先执行 mag safe path
magnetic planner 只能生成 block，不能直接控制硬件
```

### 4.4 Lock-in / OE1022D hazard

风险：

```text
输入过载
灵敏度设置错误导致长期 overload
time constant / filter 与采样窗口不匹配
状态查询打断 RALL? 高频采集
断连后 parser 阻塞
```

安全策略：

```text
OE1022D overload 必须进入 runtime status
连续 overload 超过阈值时暂停或停止 run
采集线程不做安全判断，只报告状态
executor / safety service 决定是否 stop
禁止 GUI 线程直接 RALL?
```

### 4.5 Software / data hazard

风险：

```text
未 resolved recipe 被执行
safety_report failed 仍然执行
events.jsonl 丢失
raw bin 写失败但实验继续
GUI 卡顿影响 stop
AI 生成 recipe 越权
多个 executor 同时占用同一设备
```

安全策略：

```text
executor 只接受 resolved_recipe
executor 只接受 safety_report passed
运行前必须申请 resource lease
危险命令必须写入 dry-run plan
stop / emergency stop 不依赖 GUI 刷新
安全事件必须 append-only 记录
日志写失败可触发 controlled stop
```

---

## 5. Inputs

### 5.1 Static safety configuration

```text
station_safety.json
safety_limits.schema.json
device_profile.json
station_snapshot.json
```

### 5.2 Runtime inputs

```text
resolved_recipe.json
safety_report.json
executor state
resource lease table
device connection status
device IDN / status query
overload reports
GUI arm / stop / emergency input
manual override request
```

### 5.3 Human-maintained values

以下值必须由实验室人员根据设备铭牌、手册、实验台条件和导师要求填写，不能由 AI 自动决定：

```text
laser.max_power_mw
laser.allowed_wavelength_nm
SMB100A.max_power_dbm
SMB100A.allowed_frequency_hz
SMB100A.max_fm_deviation_hz
mag.max_current_a_per_axis
mag.max_ramp_a_per_s
mag.max_hold_duration_s
OE1022D.overload_max_duration_s
emergency_stop_target_ms
safe_shutdown_timeout_ms
```

---

## 6. Outputs

### 6.1 Safety report

```text
safety_report.json
```

包含：

```text
report_id
run_id
recipe_id
schema_version
passed / failed
checked_at_wall_time
checked_at_monotonic_ns
checked_limits
dangerous_commands
rejected_steps
warnings
manual_actions_required
```

### 6.2 Runtime events

写入：

```text
events.jsonl
warnings.jsonl
```

典型事件：

```text
safety.preflight.started
safety.preflight.passed
safety.preflight.failed
safety.command.allowed
safety.command.rejected
safety.interlock.triggered
safety.manual_override.requested
safety.manual_override.accepted
safety.manual_override.rejected
safety.emergency_stop.requested
safety.safe_shutdown.started
safety.safe_shutdown.completed
safety.safe_shutdown.failed
```

### 6.3 GUI state

提供给 GUI：

```text
safety_status
armed_state
interlock_state
dangerous_command_preview
current_limits_summary
last_safety_error
emergency_stop_available
manual_override_required
```

---

## 7. Runtime Model

### 7.1 Safety service position

```text
GUI / AI / CLI
    ↓
Recipe Compiler
    ↓
Safety Validator
    ↓
safety_report.json
    ↓
Executor
    ↓
Runtime Safety Gate
    ↓
Device Drivers
```

Safety & Interlock 模块分为两部分：

```text
compile-time safety validation
runtime safety gate
```

### 7.2 Compile-time safety validation

发生在 recipe 编译后、executor 运行前。

检查：

```text
schema validity
resolved_recipe completeness
station_safety availability
all requested device values within limit
all dangerous commands listed in dry-run
all resource leases possible
all required settle / dwell present
all manual confirmations declared
```

如果失败：

```text
不生成 passed safety_report
executor 拒绝运行
GUI 显示 rejection reason
events.jsonl 写入 safety.preflight.failed
```

### 7.3 Runtime safety gate

executor 发出任何设备命令前，都必须经过：

```text
guard_command(command, current_state, resource_lease, safety_limits)
```

Runtime gate 检查：

```text
是否已 armed
是否持有设备 lease
是否处于 emergency / fault 状态
命令是否在 dry-run dangerous command list 中
命令参数是否仍在 limit 内
设备状态是否 stale
是否需要人工确认
```

### 7.4 Emergency stop path

Emergency stop 必须绕过普通队列优先执行。

推荐顺序：

```text
1. mark executor state = emergency_stopping
2. reject new commands
3. laser off / shutter close
4. SMB100A OUTP OFF, MOD OFF
5. magnetic outputs ramp to safe zero or safe hold
6. stop OE1022D acquisition or switch to passive safe mode
7. flush events.jsonl / warnings.jsonl / index.jsonl
8. write final station state query if possible
9. mark run as emergency_stopped
```

如果某设备无法响应：

```text
继续处理其他设备
记录 failure
GUI 显示 physical inspection required
run 状态不得标记为 clean completed
```

---

## 8. Data Model

### 8.1 station_safety.json

示例：

```json
{
  "schema_version": "safety_limits.v0.2",
  "station_id": "odmr_station_001",
  "updated_by": "human",
  "updated_at": "2026-05-28T00:00:00Z",
  "ai_editable": false,
  "laser": {
    "max_power_mw": 50.0,
    "allowed_wavelength_nm": [532],
    "default_state": "off",
    "requires_manual_arm": true,
    "requires_shutter_closed_on_stop": true,
    "warmup_s": 30.0,
    "settle_s": 2.0
  },
  "smb100a": {
    "max_power_dbm": 0.0,
    "allowed_frequency_hz": [[2800000000.0, 2950000000.0]],
    "max_fm_deviation_hz": 5000000.0,
    "default_rf_state": "off",
    "default_mod_state": "off",
    "requires_rf_off_before_reconfigure": true
  },
  "magnetic": {
    "max_current_a_per_axis": {
      "x": 1.0,
      "y": 1.0,
      "z": 1.0
    },
    "max_ramp_a_per_s": {
      "x": 0.1,
      "y": 0.1,
      "z": 0.1
    },
    "safe_state": "zero_current",
    "default_settle_s": 1.0
  },
  "oe1022d": {
    "overload_policy": "stop_after_threshold",
    "overload_max_duration_s": 5.0,
    "disconnect_policy": "stop_run"
  },
  "runtime": {
    "emergency_stop_target_ms": 500,
    "safe_shutdown_timeout_ms": 10000,
    "status_stale_after_ms": 2000,
    "require_resource_lease": true
  }
}
```

以上数值是格式示例，不是实际实验台推荐值。实际值必须由人工根据设备额定值和实验条件填写。

### 8.2 DangerousCommand

```json
{
  "command_id": "cmd_000123",
  "device_id": "smb100a_001",
  "kind": "smb.rf_output_on",
  "parameters": {
    "frequency_hz": 2882000000.0,
    "power_dbm": -15.0
  },
  "step_id": "step_000045",
  "requires_manual_arm": true,
  "shown_in_dry_run": true
}
```

### 8.3 SafetyDecision

```json
{
  "decision": "allow",
  "command_id": "cmd_000123",
  "checked_at_monotonic_ns": 123456789,
  "limits_applied": [
    "smb100a.max_power_dbm",
    "smb100a.allowed_frequency_hz"
  ],
  "reason": "within configured limits"
}
```

拒绝示例：

```json
{
  "decision": "reject",
  "command_id": "cmd_000124",
  "checked_at_monotonic_ns": 123456999,
  "violations": [
    {
      "limit": "smb100a.max_power_dbm",
      "requested": 10.0,
      "allowed": 0.0
    }
  ],
  "reason": "requested RF power exceeds configured station safety limit"
}
```

---

## 9. API / Interface

### 9.1 Rust safety-core crate

建议 crate：

```text
safety-core
```

主要接口：

```rust
validate_station_safety(station_safety: StationSafety) -> ValidationResult
validate_resolved_recipe(recipe: ResolvedRecipe, station_safety: StationSafety) -> SafetyReport
list_dangerous_commands(recipe: ResolvedRecipe) -> Vec<DangerousCommand>
guard_command(command: DeviceCommand, context: RuntimeSafetyContext) -> SafetyDecision
request_resource_lease(device_ids: Vec<DeviceId>, owner: RunId) -> LeaseResult
release_resource_lease(lease_id: LeaseId) -> Result
arm_run(run_id: RunId, safety_report_id: SafetyReportId, human_confirmed: bool) -> ArmResult
request_stop(run_id: RunId) -> StopResult
request_emergency_stop(reason: EmergencyReason) -> EmergencyStopResult
safe_shutdown(run_id: RunId, mode: ShutdownMode) -> ShutdownResult
```

### 9.2 Executor integration

Executor 必须调用：

```text
preflight: validate_resolved_recipe
before run: request_resource_lease + arm_run
before each command: guard_command
on stop: safe_shutdown
on error: safe_shutdown
on emergency: request_emergency_stop
```

### 9.3 GUI integration

GUI 只允许调用安全状态和控制接口：

```text
GET safety status
GET dry-run dangerous commands
POST arm run
POST stop run
POST emergency stop
POST manual override request
```

GUI 禁止：

```text
直接打开 laser
直接打开 SMB RF
直接设置 mag current
直接访问底层 serial / TCP / VISA
修改 station_safety hard limit
```

### 9.4 AI agent integration

AI agent 只允许：

```text
生成 recipe / block
解释 safety_report
指出可能不安全的参数
建议降低功率 / 电流 / ramp rate
生成 review checklist
```

AI agent 禁止：

```text
修改 station_safety.json
提高 hard limit
绕过 failed safety_report
直接调用 executor
直接访问硬件
自动确认 manual arm
自动执行 manual override
```

---

## 10. Safety Rules

### 10.1 Global rules

```text
S-001: executor 不接受未 resolved 的 recipe
S-002: executor 不接受 schema 未通过的 recipe
S-003: executor 不接受 safety_report failed 的 recipe
S-004: safety_report 必须和 resolved_recipe hash 绑定
S-005: station_safety 必须和 run 绑定并写入 station_snapshot.json
S-006: dangerous command 必须出现在 dry-run 中
S-007: dangerous command 执行前必须 guard_command allow
S-008: 所有设备动作必须持有 resource lease
S-009: AI 不能修改任何 safety limit
S-010: recipe 不能覆盖 station safety limit
S-011: unknown device state = unsafe
S-012: stale status = unsafe
S-013: stop / emergency stop 必须高于普通命令优先级
S-014: safe shutdown 失败必须标记 run 为 unsafe_terminated
```

### 10.2 Laser rules

```text
L-001: laser 默认 off
L-002: laser enable 属于 dangerous command
L-003: shutter open 属于 dangerous command
L-004: set power above zero 属于 dangerous command
L-005: requested power <= laser.max_power_mw
L-006: wavelength must be allowed by station safety profile
L-007: laser on 前必须 human arm
L-008: stop / emergency 必须关闭 laser 或 shutter
```

### 10.3 SMB100A rules

```text
M-001: RF 默认 off
M-002: OUTP ON 属于 dangerous command
M-003: MOD ON 属于 dangerous command when RF output can be affected
M-004: power set above previous power while RF on 属于 dangerous command
M-005: frequency must be inside allowed_frequency_hz
M-006: power <= max_power_dbm
M-007: FM deviation <= max_fm_deviation_hz
M-008: sweep start / stop / step / dwell must be dry-run visible
M-009: RF on 前必须确认 frequency and power
M-010: RF reconfigure 默认要求 RF off，除非 profile 明确允许
```

### 10.4 Magnetic rules

```text
B-001: current per axis <= max_current_a_per_axis
B-002: ramp per axis <= max_ramp_a_per_s
B-003: vector combined limit must pass if configured
B-004: target current 必须经过 Magnetic Planner 或等价 safety check
B-005: emergency stop 使用 zero_current 或 configured safe_hold
B-006: ramp path 必须可 dry-run 展示
B-007: settle time 必须由 compiler 插入 resolved_recipe
B-008: manual direct current control 必须记录 reason
```

### 10.5 OE1022D rules

```text
O-001: overload status 必须记录
O-002: continuous overload > threshold 触发 stop 或 pause
O-003: OE1022D disconnect during run 触发 controlled stop
O-004: GUI 不得直接查询 RALL?
O-005: safety status polling 不得打断 acquisition thread
```

---

## 11. Error Handling

### 11.1 Device disconnect

| Device | Default response |
|---|---|
| Laser | emergency stop, physical inspection required |
| SMB100A | attempt OUTP OFF if possible, mark unsafe if no confirmation |
| Magnetic controller | attempt safe zero, mark unsafe if no confirmation |
| OE1022D | controlled stop, preserve raw bin and events |

### 11.2 Command timeout

```text
普通命令 timeout → controlled stop
危险命令 timeout → emergency stop path
safe shutdown command timeout → continue other shutdown commands, mark device unknown
```

### 11.3 Safety report mismatch

如果运行时发现：

```text
resolved_recipe hash != safety_report recipe_hash
station_safety hash != safety_report station_safety_hash
```

必须：

```text
reject run
record safety.preflight.failed
require recompile + revalidate
```

### 11.4 Log write failure

如果 events.jsonl 无法写入：

```text
before arm: reject run
during run: controlled stop unless emergency path is already active
after emergency: try fallback emergency log file
```

### 11.5 Manual override failure

manual override 不允许突破 hard limit。

如果 override 请求试图突破 hard limit：

```text
reject
record safety.manual_override.rejected
show reason
```

---

## 12. Resource Lease Model

### 12.1 Why resource lease is required

多设备系统中不能允许两个任务同时控制同一设备。

```text
GUI status panel
executor run
manual control panel
agent dry-run
```

必须通过 resource lease 决定谁有权发命令。

### 12.2 Lease types

```text
read_only
control
exclusive_run
emergency
```

优先级：

```text
emergency > exclusive_run > control > read_only
```

### 12.3 Lease rules

```text
RL-001: executor run 必须持有 exclusive_run lease
RL-002: GUI manual panel 只能申请 control lease
RL-003: status monitor 只能申请 read_only lease
RL-004: emergency stop 可抢占所有 lease
RL-005: lease acquire / release 必须记录 events
RL-006: stale lease 必须可 recovery
```

---

## 13. Manual Override Rules

### 13.1 Allowed override

人工 override 只允许：

```text
降低限制
延长 settle time
降低 laser power
降低 RF power
降低 mag current
延长 ramp duration
停止或跳过某 step
手动确认已完成物理安全检查
```

### 13.2 Forbidden override

人工 override 也不能：

```text
突破 hard safety limit
绕过 emergency stop
让 AI 自动确认安全
执行未出现在 dry-run 中的危险命令
让 GUI 直接访问硬件
删除安全事件日志
```

### 13.3 Override record

每次 override 必须记录：

```text
override_id
operator_id or local_user
reason
requested_change
previous_value
new_value
affected_run_id
affected_step_id
wall_time
monotonic_time
```

---

## 14. Test Harness

### 14.1 Required mocks

```text
fake laser serial
fake SMB100A SCPI server
fake mag axis controller
fake OE1022D overload reporter
fake resource lease manager
fake event logger
```

### 14.2 Required tests

```text
schema validation tests
safety limit rejection tests
dangerous command list tests
dry-run visibility tests
resource lease contention tests
emergency stop tests
safe shutdown tests
device disconnect tests
command timeout tests
OE1022D overload tests
SMB RF power rejection tests
laser power rejection tests
mag current rejection tests
mag ramp rejection tests
manual override rejection tests
safety report hash mismatch tests
log failure tests
```

### 14.3 Fault injection cases

```text
SMB100A accepts command but query fails
laser disconnects after enable
mag axis ignores zero command
OE1022D overload toggles rapidly
events.jsonl write permission denied
GUI emergency stop clicked during serial timeout
executor crashes after RF on but before normal stop
station_safety changed after safety_report generated
```

---

## 15. Acceptance Criteria

### 15.1 Architecture acceptance

```text
AC-001: 所有危险命令必须经过 safety-core guard_command
AC-002: executor 无法运行 safety_report failed 的 recipe
AC-003: safety_report 与 resolved_recipe hash 绑定
AC-004: station_safety snapshot 写入 run directory
AC-005: GUI 不存在直接硬件访问路径
AC-006: AI agent 不存在 safety limit 修改路径
```

### 15.2 Runtime acceptance

```text
AC-101: emergency stop 可在任意 run 状态下触发
AC-102: emergency stop 后系统拒绝新普通命令
AC-103: emergency stop 尝试关闭 laser / SMB RF / mag output
AC-104: emergency stop 结果写入 events.jsonl
AC-105: 设备状态未知时 run 不被标记为 clean completed
AC-106: OE1022D overload 超阈值时触发预设策略
```

### 15.3 Testing acceptance

```text
AC-201: fake SMB100A 能验证 RF power / OUTP ON rejection
AC-202: fake laser 能验证 max power / shutter rejection
AC-203: fake mag 能验证 current / ramp rejection
AC-204: fake OE1022D 能验证 overload stop
AC-205: resource lease contention 测试通过
AC-206: safe shutdown fault injection 测试通过
AC-207: emergency stop path 覆盖率达到项目设定阈值
```

### 15.4 Documentation acceptance

```text
AC-301: 所有 safety limit 字段有单位
AC-302: 所有 dangerous command 类型有定义
AC-303: 所有 manual override 类型有记录格式
AC-304: 所有 safety rejection 都能显示给 GUI
AC-305: 所有安全事件都能回溯到 run_id / step_id / device_id
```

---

## 16. Agent Constraints

### 16.1 AI agent allowed

AI agent 可以：

```text
生成 recipe
生成 block
检查 recipe 是否可能违反 safety
解释 safety_report
建议降低参数
生成 review checklist
生成 mock tests
生成文档
```

### 16.2 AI agent forbidden

AI agent 禁止：

```text
修改 station_safety.json
提高 max_power / max_current / max_ramp
自动点击 arm
自动确认 manual override
直接调用 executor run
直接访问 hardware driver
删除 events.jsonl
修改 emergency stop 逻辑
```

### 16.3 Agent review checklist

任何 agent 修改相关代码或 schema 时必须检查：

```text
是否新增 dangerous command
是否新增 safety limit 字段
是否改变 default safe state
是否绕过 guard_command
是否允许 GUI 直接访问硬件
是否允许 recipe 覆盖 safety limit
是否影响 emergency stop path
是否增加测试覆盖
```

---

## 17. Open Questions

```text
Q-001: 实际 laser max power 由哪份实验室记录确认？
Q-002: 是否存在物理 shutter 或只能通过 laser serial disable？
Q-003: SMB100A 最大 RF power 是否按设备最大值、天线限制还是样品限制设定？
Q-004: 磁场电源是否支持硬件限流和远程查询？
Q-005: mag safe state 是 zero_current 还是 safe_hold？
Q-006: emergency stop target ms 目标值应由硬件响应测试决定。
Q-007: OE1022D overload 字段是否能在 RALL? frame 或状态查询中稳定获得？
Q-008: 是否需要独立硬件急停按钮，而不是只靠软件按钮？
```

---

## 18. External References

These references are used only to frame safety requirements. Actual limits must be set from local device manuals, instrument labels, laboratory rules, and supervisor approval.

```text
Rohde & Schwarz R&S SMB100A Operating Manual, Version 23:
  covers SMB100A RF configuration, modulation, sweep/list modes, remote control, and safety documentation.

IEC 60825-1:2014 Laser product safety standard:
  global laser product safety classification and requirements reference.

Zurich Instruments MFLI User Manual:
  reference for lock-in amplifier operation, input/output limits, and overload-related instrument behavior.
```

---

## 19. v0.2 Deliverables

```text
10_safety_interlock_prd.md
safety_limits.schema.json draft
safety_report.schema.json draft
DangerousCommand enum draft
SafetyDecision enum draft
fake safety-core tests list
emergency stop sequence diagram
resource lease state machine diagram
manual override checklist
```


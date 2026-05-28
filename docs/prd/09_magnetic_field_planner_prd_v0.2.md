# Sub-PRD 09: Magnetic Field Planner

Version: v0.2  
Status: Draft for architecture freeze  
Target file: `docs/prd/09_magnetic_field_planner_prd.md`  
Related PRDs: `00_main_prd`, `01_architecture_prd`, `04_recipe_json_schema_prd`, `05_recipe_compiler_executor_prd`, `06_timing_sync_averaging_prd`, `10_safety_interlock_prd`, `11_harness_mock_replay_prd`, `12_agent_workflow_prd`

---

## 1. Purpose

Magnetic Field Planner 负责把实验人员或 AI agent 给出的磁场目标转换为可验证、可预览、可执行的磁场控制 block。

本模块解决的问题是：

```text
用户想设置或扫描 B field
    ↓
系统需要把 B vector / spherical B / grid sweep / path sweep
    ↓
转换为 coil current
    ↓
检查 current limit / ramp limit / settle time / zero offset
    ↓
生成 magnetic block JSON
    ↓
交给 recipe compiler 合并进 resolved_recipe
```

它不是完整磁场仿真软件，而是 ODMR 自动化系统中的磁场规划、安全检查和 recipe 生成模块。

---

## 2. Scope

### 2.1 本模块负责

```text
B vector 表达
球坐标表达
笛卡尔坐标表达
磁场单位转换
coil matrix 应用
zero offset 修正
current limit 检查
ramp limit 检查
settle time 估算
grid sweep 生成
spherical sweep 生成
path sweep 生成
preview coil current
preview B path
生成 magnetic block JSON
导出到 recipe / block
```

### 2.2 本模块不直接控制硬件

Magnetic Field Planner 只做规划和校验：

```text
输入: 目标 B field / sweep 参数 / station calibration
输出: magnetic block JSON / preview / safety report
```

真正发送电流命令由 Executor 执行。

### 2.3 本模块在系统中的位置

```text
Web GUI Magnetic Planner page
        ↓
Rust magnetic-planner core
        ↓
magnetic_block.json
        ↓
Recipe Compiler
        ↓
resolved_recipe.json
        ↓
Executor
        ↓
Mag X/Y/Z device driver
```

---

## 3. Non-goals

第一阶段不做：

```text
复杂三维磁场仿真 Web App
三维场分布 FEM 仿真
线圈几何自动优化
NV 轴自动反演
磁场反演模型训练
空间非均匀场建模
真实时间闭环磁场控制
AI 直接调磁场硬件
```

第一阶段只做：

```text
磁场向量规划
线圈电流预估
安全检查
扫描路径展开
JSON block 生成
dry-run 预览
```

---

## 4. Coordinate and Unit Convention

### 4.1 默认坐标系

系统默认使用实验室坐标系：

```text
+X: mag_x coil positive direction
+Y: mag_y coil positive direction
+Z: mag_z coil positive direction
```

具体方向必须由 station calibration 文件定义，不能只依赖线圈物理标签。

### 4.2 笛卡尔表达

```json
{
  "type": "cartesian",
  "unit": "mT",
  "bx": 0.0,
  "by": 0.0,
  "bz": 2.0
}
```

内部标准单位：

```text
B field: tesla, T
current: ampere, A
angle: rad internally, deg allowed at JSON boundary
```

### 4.3 球坐标表达

本项目采用物理常用约定：

```text
B_abs = |B|
theta = angle from +Z axis
phi = azimuth from +X axis toward +Y axis
```

转换关系：

```text
Bx = B_abs * sin(theta) * cos(phi)
By = B_abs * sin(theta) * sin(phi)
Bz = B_abs * cos(theta)
```

JSON 示例：

```json
{
  "type": "spherical",
  "unit": "mT",
  "b_abs": 3.0,
  "theta_deg": 54.7,
  "phi_deg": 45.0
}
```

### 4.4 NV 轴表达

第一阶段只允许把 NV axis 作为 metadata 或 display helper，不参与自动反演。

```json
{
  "nv_axis_hint": {
    "enabled": true,
    "basis": "lab_cartesian",
    "unit_vector": [0.57735, 0.57735, 0.57735]
  }
}
```

Planner 可以显示：

```text
B_parallel = dot(B, nv_axis)
B_perpendicular = sqrt(|B|^2 - B_parallel^2)
```

但不能把该结果作为 safety limit 或硬件命令来源。

---

## 5. Inputs

### 5.1 Station magnetic calibration

来自 `station.json` 或 `station_snapshot.json`：

```json
{
  "magnetic_system": {
    "type": "three_axis_coil",
    "axes": ["x", "y", "z"],
    "field_unit": "T",
    "current_unit": "A",
    "coil_matrix_t_per_a": [
      [0.0010, 0.0000, 0.0000],
      [0.0000, 0.0010, 0.0000],
      [0.0000, 0.0000, 0.0010]
    ],
    "zero_offset_t": [0.0, 0.0, 0.0],
    "current_offset_a": [0.0, 0.0, 0.0],
    "max_current_a": [2.0, 2.0, 2.0],
    "max_ramp_rate_a_per_s": [0.2, 0.2, 0.2],
    "default_settle_ms": 500
  }
}
```

### 5.2 User / agent requested magnetic plan

```json
{
  "magnetic_plan": {
    "mode": "grid_sweep",
    "basis": "lab_cartesian",
    "unit": "mT",
    "bx": { "start": -1.0, "stop": 1.0, "points": 11 },
    "by": { "value": 0.0 },
    "bz": { "value": 2.0 },
    "settle_ms": 800
  }
}
```

### 5.3 Safety policy

来自 `safety_limits.json` 或 `station.json` 中的 locked section：

```json
{
  "safety_limits": {
    "magnetic": {
      "max_abs_b_t": 0.01,
      "max_current_a": [2.0, 2.0, 2.0],
      "max_ramp_rate_a_per_s": [0.2, 0.2, 0.2],
      "forbid_unverified_calibration": true
    }
  }
}
```

AI-generated recipe 不允许覆盖 safety limits。

---

## 6. Outputs

### 6.1 Magnetic block JSON

Planner 的主要输出是可插入 recipe 的 block：

```json
{
  "schema_version": "0.2",
  "block_type": "magnetic_sweep",
  "block_id": "mag_bz_grid_001",
  "basis": "lab_cartesian",
  "points": [
    {
      "point_id": "b_0000",
      "b_target_t": [0.0, 0.0, 0.0010],
      "coil_current_target_a": [0.0, 0.0, 1.0],
      "settle_ms": 800,
      "estimated_ramp_ms": 5000,
      "safety_status": "pass"
    }
  ],
  "safety_summary": {
    "status": "pass",
    "max_abs_b_t": 0.0010,
    "max_abs_current_a": 1.0,
    "max_ramp_rate_a_per_s": 0.2
  }
}
```

### 6.2 Preview payload

供 GUI 渲染：

```json
{
  "preview": {
    "num_points": 101,
    "estimated_total_time_s": 180.5,
    "b_path": [[0.0, 0.0, 0.001], [0.0, 0.0, 0.0011]],
    "coil_current_path": [[0.0, 0.0, 1.0], [0.0, 0.0, 1.1]],
    "warnings": [],
    "errors": []
  }
}
```

### 6.3 Safety report fragment

```json
{
  "magnetic_safety_report": {
    "status": "pass",
    "checks": [
      {
        "check": "current_limit",
        "status": "pass",
        "max_current_a": [0.5, 0.2, 1.1]
      },
      {
        "check": "ramp_limit",
        "status": "pass"
      }
    ]
  }
}
```

---

## 7. Runtime Model

### 7.1 Planner runtime

Planner 运行在 Rust backend 中，不运行在前端。

```text
Tauri Web UI
  invokes
Rust command: magnetic.preview(plan)
  calls
magnetic-planner crate
  reads
station calibration + safety limits
  returns
preview + magnetic_block_json + safety_report_fragment
```

### 7.2 Executor runtime

Executor 运行时只执行已经编译好的 resolved step：

```text
resolved step: set magnetic current
    ↓
Executor validates resource lease
    ↓
Mag driver command queue
    ↓
current ramp / setpoint command
    ↓
events.jsonl records command sent / ack / settle start / settle done
```

Planner 不参与真实运行过程。

---

## 8. Data Model

### 8.1 Magnetic field target

```json
{
  "b_target_t": [0.0, 0.0, 0.002]
}
```

### 8.2 Coil matrix

定义：

```text
B = M * (I - I_offset) + B_zero_offset
```

因此：

```text
I = inv(M) * (B_target - B_zero_offset) + I_offset
```

如果 `M` 不可逆，Planner 必须拒绝。

### 8.3 Magnetic sweep point

```json
{
  "point_id": "b_0123",
  "b_target_t": [0.0001, 0.0, 0.0020],
  "coil_current_target_a": [0.1, 0.0, 2.0],
  "ramp_from_previous": {
    "delta_current_a": [0.01, 0.0, 0.02],
    "min_ramp_time_ms": 100
  },
  "settle_ms": 800,
  "metadata": {
    "source": "grid_sweep",
    "index": [1, 0, 20]
  }
}
```

### 8.4 Magnetic block types

第一阶段支持：

```text
single_field
grid_sweep
spherical_sweep
path_sweep
```

暂不支持：

```text
closed_loop_field_control
field_map_inversion
adaptive_magnetic_search
```

---

## 9. Sweep Modes

### 9.1 Single field

用于设置固定磁场：

```json
{
  "mode": "single_field",
  "target": {
    "type": "cartesian",
    "unit": "mT",
    "bx": 0.0,
    "by": 0.0,
    "bz": 2.0
  }
}
```

### 9.2 Cartesian grid sweep

用于扫 Bx/By/Bz：

```json
{
  "mode": "grid_sweep",
  "basis": "lab_cartesian",
  "unit": "mT",
  "axes": {
    "bx": { "start": -1.0, "stop": 1.0, "points": 21 },
    "by": { "value": 0.0 },
    "bz": { "values": [0.5, 1.0, 1.5, 2.0] }
  },
  "order": ["bz", "bx", "by"]
}
```

`order` 决定展开顺序，也影响 ramp 时间。

### 9.3 Spherical sweep

用于固定 |B|，扫描方向：

```json
{
  "mode": "spherical_sweep",
  "unit": "mT",
  "b_abs": 3.0,
  "theta_deg": { "start": 0.0, "stop": 90.0, "points": 19 },
  "phi_deg": { "values": [0.0, 45.0, 90.0] },
  "order": ["phi", "theta"]
}
```

### 9.4 Path sweep

用于人工指定路径：

```json
{
  "mode": "path_sweep",
  "unit": "mT",
  "points": [
    { "bx": 0.0, "by": 0.0, "bz": 1.0 },
    { "bx": 0.0, "by": 0.0, "bz": 1.2 },
    { "bx": 0.1, "by": 0.0, "bz": 1.2 }
  ]
}
```

Path sweep 不会自动排序。Planner 只检查安全和 ramp。

---

## 10. API / Interface

### 10.1 Rust crate

Crate name:

```text
magnetic-planner-core
```

Public interfaces:

```rust
pub fn normalize_field(input: MagneticFieldInput) -> Result<BVectorT>;
pub fn field_to_current(b: BVectorT, calibration: CoilCalibration) -> Result<CurrentVectorA>;
pub fn expand_sweep(plan: MagneticPlan) -> Result<Vec<MagneticPoint>>;
pub fn check_magnetic_safety(points: &[MagneticPoint], limits: SafetyLimits) -> SafetyReport;
pub fn build_magnetic_block(plan: MagneticPlan, station: StationSnapshot) -> Result<MagneticBlock>;
pub fn preview_magnetic_plan(plan: MagneticPlan, station: StationSnapshot) -> Result<MagneticPreview>;
```

### 10.2 Tauri commands

```text
magnetic_preview_plan(plan_json) -> preview_json
magnetic_generate_block(plan_json) -> magnetic_block_json
magnetic_validate_block(block_json) -> safety_report_json
magnetic_export_block(block_json, path) -> result
```

### 10.3 Compiler interface

Recipe Compiler 接收：

```json
{
  "include_block": "mag_bz_grid_001.block.json"
}
```

或 inline block：

```json
{
  "block_type": "magnetic_sweep",
  "block_id": "mag_bz_grid_001",
  "points": []
}
```

Compiler 负责把 magnetic points 与 SMB/OE1022D/laser steps 合并为 resolved_recipe。

---

## 11. GUI Requirements

Magnetic Planner 应作为独立 GUI 子页面，而不是塞在主运行页中。

### 11.1 页面区域

```text
Magnetic Planner Page
  - Calibration summary panel
  - Coordinate input panel
  - Sweep mode panel
  - Safety limits panel, read-only
  - Preview current table
  - Preview B path chart
  - Estimated time panel
  - Export block button
  - Add to recipe button
```

### 11.2 GUI 不允许

```text
GUI 不直接访问磁场硬件
GUI 不直接发送 current command
GUI 不允许编辑 locked safety limit
GUI 不允许绕过 planner safety check
GUI 不允许生成未带 calibration hash 的 magnetic block
```

### 11.3 预览必须显示

```text
每个点的 Bx/By/Bz
每个点的 Ix/Iy/Iz
最大电流
最大 ramp
预计 ramp time
settle time
unsafe point index
calibration id / hash
```

---

## 12. Safety Rules

### 12.1 强制检查

Planner 必须执行：

```text
coil_matrix exists
coil_matrix invertible
calibration status verified
B target finite
current target finite
abs(B) below max_abs_b_t
current below per-axis max_current_a
ramp below max_ramp_rate_a_per_s, or ramp time inserted
settle_ms >= minimum_settle_ms
point count below maximum_points
estimated runtime below optional maximum_run_time
```

### 12.2 拒绝条件

任何一项成立时必须拒绝生成 runnable block：

```text
缺失 coil matrix
coil matrix singular or near-singular
未校准且 safety policy forbid_unverified_calibration = true
任何轴电流超过限制
任何点 B_abs 超过限制
NaN / Inf / null 出现在目标磁场或电流中
AI recipe 尝试覆盖 safety limit
```

### 12.3 Warning 条件

以下情况允许生成 block，但必须 warning：

```text
预计运行时间很长
point count 很大
ramp 时间远大于采集时间
磁场路径非单调
spherical theta/phi 经过坐标奇点
current 接近 limit，例如 > 90%
```

---

## 13. Error Handling

### 13.1 Planner errors

```text
E_MAG_CALIBRATION_MISSING
E_MAG_MATRIX_SINGULAR
E_MAG_FIELD_OUT_OF_RANGE
E_MAG_CURRENT_LIMIT_EXCEEDED
E_MAG_RAMP_LIMIT_EXCEEDED
E_MAG_INVALID_COORDINATE
E_MAG_UNSUPPORTED_SWEEP_MODE
E_MAG_UNSAFE_AI_OVERRIDE
```

### 13.2 Runtime errors

Executor 层处理：

```text
E_MAG_DEVICE_NOT_CONNECTED
E_MAG_COMMAND_TIMEOUT
E_MAG_CURRENT_SET_FAILED
E_MAG_DRIVER_REJECTED_LIMIT
E_MAG_SETTLE_TIMEOUT
E_MAG_EMERGENCY_STOP
```

这些错误应写入 `events.jsonl`。

---

## 14. Test Harness

### 14.1 Unit tests

```text
cartesian input normalization
spherical to cartesian conversion
unit conversion mT/uT/T
coil matrix inversion
zero offset correction
current limit check
ramp time estimation
grid sweep expansion
spherical sweep expansion
path sweep validation
safety rejection
```

### 14.2 Mock tests

```text
fake mag axis driver
fake station calibration
fake unsafe calibration
fake singular coil matrix
fake current-limit violation
fake ramp-limit violation
```

### 14.3 Integration tests

```text
magnetic block generated from GUI-style input
magnetic block accepted by recipe compiler
resolved_recipe contains magnetic steps
executor dry-run shows current commands
events.jsonl records magnetic setpoint events
```

### 14.4 Golden files

```text
tests/golden/mag_single_field.block.json
tests/golden/mag_bz_grid.block.json
tests/golden/mag_spherical_sweep.block.json
tests/golden/mag_path_sweep.block.json
tests/golden/mag_safety_failed.report.json
```

---

## 15. Acceptance Criteria

v0.2 通过标准：

```text
能够输入单个 B vector 并生成 coil current
能够生成 grid sweep magnetic block
能够生成 spherical sweep magnetic block
能够生成 path sweep magnetic block
能够检查 current limit
能够检查 ramp limit
能够估算 settle / ramp time
能够导出 magnetic_block.json
能够被 recipe compiler 引用
GUI preview 不直接访问硬件
AI agent 不能覆盖 safety limit
unsafe magnetic plan 必须被拒绝
测试覆盖 conversion / matrix / safety / sweep expansion
```

---

## 16. Agent Constraints

### 16.1 Magnetic Planner Agent 可以做

```text
根据用户目标生成 magnetic_plan JSON
解释 B vector / theta / phi / coil current preview
生成 grid sweep / spherical sweep / path sweep 参数
检查 block 是否符合 schema
建议扫描范围和点数
根据 dry-run report 修改 recipe
```

### 16.2 Magnetic Planner Agent 不可以做

```text
不直接访问磁场硬件
不直接发送 current command
不修改 safety limit
不忽略 current limit warning
不绕过 calibration verification
不把 unverified calibration 标记为 verified
不在真实运行中动态改变磁场计划
```

### 16.3 Agent review checklist

Agent 生成 magnetic block 前必须检查：

```text
是否声明坐标系
是否声明单位
是否使用 station calibration
是否包含 settle time
是否包含 point count
是否触发 current limit
是否触发 ramp limit
是否生成 safety summary
是否能被 recipe compiler 展开
```

---

## 17. Open Questions

```text
实际三轴线圈的 coil_matrix 如何标定？
zero_offset 是否只来自地磁场，还是包括电源零偏？
是否需要记录环境磁场传感器读数？
磁场 settle time 是固定值、经验模型，还是后期由测量反馈估计？
NV axis hint 是否需要在 v0.3 进入正式数据模型？
是否需要对大规模 spherical sweep 做路径排序优化以减少 ramp time？
```

---

## 18. v0.3 Candidates

```text
路径排序优化
NV axis projection view
B_parallel / B_perpendicular based scan helper
simple 2D heatmap preview
calibration wizard
magnetic field sensor integration
closed-loop magnetic stabilization
active-learning magnetic scan planner
```

---

## 19. Summary

Magnetic Field Planner 的核心边界是：

```text
规划磁场
预览电流
检查安全
生成 block
不碰硬件
不绕过 safety
```

它应被视为一个独立子系统，而不是 GUI 的一个小表单。第一阶段的目标不是做复杂磁场仿真，而是保证每个进入 recipe 的磁场点都可追溯、可校验、可 dry-run、可复现。

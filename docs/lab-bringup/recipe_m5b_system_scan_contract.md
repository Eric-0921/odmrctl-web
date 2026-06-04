# Recipe-M5B-A System-Level Scan Contract

> 文档定位：M5B 系统级扫描 Recipe 合约定义
> 所属项目：odmrctl-web / ODMR Automation
> 依赖 PRD：04_recipe_json_schema_prd_v0.2, 09_magnetic_field_planner_prd_v0.2, 05_recipe_compiler_executor_prd_v0.2, 10_safety_interlock_prd_v0.2
> 创建日期：2026-05-31

---

## 1. 目的

本文档定义 `system_scan_recipe` 的 JSON 合约，支持以下系统级参数扫描：

- SMB100A RF 频率/功率设置与扫频
- Maynuo M8812 三轴磁场矢量设置与磁扫描
- OE1022D 采集窗口
- 可选激光器占位块（默认禁用）

该合约把实验意图从设备命令中分离，确保：
- 所有参数带单位语义
- 安全上限不可被 recipe 覆盖
- 每个数据点可回溯到 step_id、sweep_coordinates、recipe_hash
- AI 只能生成意图级 JSON，不能直接生成 SCPI/串口命令

---

## 2. 顶层元数据

每个 `system_scan_recipe` 必须包含：

```json
{
  "schema_version": "0.2.0",
  "kind": "system_scan_recipe",
  "id": "m5b_rf_mag_oe_system_scan",
  "description": "System-level RF + magnetic + OE scan recipe",
  "station_ref": "examples/preflight/station.example.json",
  "physical_response_required": false
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `schema_version` | string | ✅ | 格式 `MAJOR.MINOR.PATCH` |
| `kind` | string | ✅ | 固定为 `"system_scan_recipe"` |
| `id` | string | ✅ | 小写、数字、下划线、短横线 |
| `description` | string | ✅ | 人类可读描述 |
| `station_ref` | string | ✅ | 指向 station JSON 的路径或 ID |
| `physical_response_required` | bool | ✅ | 是否需要物理响应确认 |

---

## 3. 设备引用 (devices)

```json
{
  "devices": {
    "smb100a": { "device_id": "smb100a_main", "required": true },
    "oe1022d": { "device_id": "oe1022d_main", "required": true },
    "magnetic": { "device_id": "maynuo_m8812_axes", "required": true },
    "laser": { "device_id": "cni_laser", "required": false, "enabled": false }
  }
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `device_id` | string | ✅ | 设备实例标识 |
| `required` | bool | ✅ | 执行时是否必须连接 |
| `enabled` | bool | 可选 | 仅 laser 使用；默认 `false` |

---

## 4. 固定参数 (fixed_params)

固定参数在每个 sweep point 保持不变，由 compiler 写入每个 resolved step 的 `target_device_state`。

```json
{
  "fixed_params": {
    "smb100a": {
      "rf_power_dbm": -30.0,
      "fm_enabled": true,
      "fm_deviation_hz": 4000000,
      "mod_enabled": true
    },
    "magnetic": {
      "mode": "field_vector",
      "unit": "nT",
      "basis": "lab_cartesian",
      "default_settle_ms": 500
    },
    "laser": {
      "enabled": false,
      "power_mw": 0.0,
      "settle_ms": 0
    },
    "oe1022d": {
      "frames_per_point": 5,
      "inter_frame_delay_ms": 20
    }
  }
}
```

**规则：**
- 所有数值字段必须带单位语义（`_dbm`, `_hz`, `_ms`, `_mw`, `_nt`）
- 禁止模糊字段名如 `freq`, `power`, `level`
- 禁止声明安全上限
- 禁止声明设备命令数组

---

## 5. 扫描参数 (sweeps)

### 5.1 磁场笛卡尔网格扫描

```json
{
  "sweep_id": "mag_z_low_current_points",
  "device": "magnetic",
  "axis_group": "magnetic.vector",
  "type": "cartesian_grid",
  "unit": "nT",
  "axes": {
    "bx_nt": { "value": 0.0 },
    "by_nt": { "value": 0.0 },
    "bz_nt": { "values": [-1000.0, 0.0, 1000.0] }
  }
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `sweep_id` | string | ✅ | 唯一标识 |
| `device` | string | ✅ | `"magnetic"` |
| `axis_group` | string | ✅ | `"magnetic.vector"` |
| `type` | string | ✅ | `"cartesian_grid"` |
| `unit` | string | ✅ | `"nT"` / `"uT"` / `"mT"` / `"T"` |
| `axes` | object | ✅ | 每个轴可以是 `value`（固定）或 `values`（列表） |

**展开规则：**
- 带有 `values` 的轴为扫描轴
- 带有 `value` 的轴为固定轴
- 多个 `values` 轴的笛卡尔积构成所有点
- 当前 M5B-A 仅支持单轴扫描（其余轴固定）

### 5.2 RF 频率值列表扫描

```json
{
  "sweep_id": "rf_frequency_points",
  "device": "smb100a",
  "axis": "smb100a.rf.frequency_hz",
  "values": [2878000000, 2882000000, 2886000000]
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `sweep_id` | string | ✅ | 唯一标识 |
| `device` | string | ✅ | `"smb100a"` |
| `axis` | string | ✅ | 轴标识符，带点号路径 |
| `values` | [number] | ✅ | 离散频率值列表 |

---

## 6. 扫描顺序 (sweep_order)

```json
{
  "sweep_order": ["mag_z_low_current_points", "rf_frequency_points"]
}
```

**语义：**
- 外层循环 = `sweep_order` 中靠前的 sweep
- 内层循环 = `sweep_order` 中靠后的 sweep
- 上述示例：每个磁场点下扫完整 RF 频率列表
- 总点数 = 各 sweep 点数的乘积

---

## 7. 采集策略 (acquisition_policy)

```json
{
  "acquisition_policy": {
    "enabled": true,
    "device": "oe1022d",
    "mode": "per_final_sweep_point",
    "start_after": [
      "magnetic_settled",
      "rf_configured",
      "rf_output_on_confirmed"
    ],
    "pre_discard_ms": 100,
    "frames_per_point": 5,
    "attach_device_state_snapshot": true
  }
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `enabled` | bool | ✅ | 是否采集 |
| `device` | string | ✅ | 采集设备标识 |
| `mode` | string | ✅ | `"per_final_sweep_point"` |
| `start_after` | [string] | ✅ | 采集启动前置条件列表 |
| `pre_discard_ms` | number | ✅ | 前置丢弃时间 |
| `frames_per_point` | number | ✅ | 每点采集帧数 |
| `attach_device_state_snapshot` | bool | ✅ | 是否附加设备状态快照 |

**采集启动语义：**
- 仅在最终 sweep point（所有 sweep 轴的当前值组合）触发采集
- 中间嵌套层不触发单独采集
- 前置条件全部满足后才打开采集窗口

---

## 8. 安全声明 (safety)

```json
{
  "safety": {
    "require_operator_approval": true,
    "no_internal_smb_sweep": true,
    "no_realtime_csv": true,
    "no_gui_direct_hardware": true,
    "laser_default_disabled": true
  }
}
```

**规则：**
- `safety` 块仅声明策略开关，不定义数值上限
- 所有数值上限来自 `station_ref` 指向的 station/safety profile
- recipe 中禁止出现 `safety_limits` 数值覆盖

---

## 9. Resolved Recipe Step Model

Compiler 展开后的每个 step：

```json
{
  "step_id": "pt_000_mag_000_rf_000",
  "point_index": 0,
  "phase": "measure",
  "sweep_coordinates": {
    "mag_z_low_current_points.bz_nt": -1000.0,
    "rf_frequency_points.frequency_hz": 2878000000
  },
  "target_device_state": {
    "magnetic": {
      "b_target_nt": [0.0, 0.0, -1000.0],
      "settle_ms": 500
    },
    "smb100a": {
      "frequency_hz": 2878000000,
      "rf_power_dbm": -30.0,
      "rf_output_required": true
    },
    "laser": {
      "enabled": false
    }
  },
  "acquisition": {
    "enabled": true,
    "device": "oe1022d",
    "frames_expected": 5
  },
  "traceability": {
    "source_recipe_id": "m5b_rf_mag_oe_system_scan",
    "required_state_snapshot": true,
    "required_step_hash": true
  }
}
```

**必须字段：**
- `step_id` — 全局唯一，稳定排序
- `point_index` — 点在总序列中的索引
- `phase` — `"setup"` / `"measure"` / `"cleanup"`
- `sweep_coordinates` — 该点在所有 sweep 轴上的坐标
- `target_device_state` — 该 step 期望的设备状态
- `traceability` — 可追溯性元数据

---

## 10. Safety Report Model

```json
{
  "schema_version": "0.2.0",
  "kind": "safety_report",
  "id": "safety_m5b_20260531_001",
  "resolved_recipe_id": "resolved_m5b_rf_mag_oe_system_scan",
  "decision": "allow",
  "requires_operator_approval": true,
  "physical_response_required": false,
  "checks": [
    { "check": "recipe_schema_valid", "status": "pass" },
    { "check": "station_ref_valid", "status": "pass" },
    { "check": "safety_limits_not_overridden", "status": "pass" },
    { "check": "smb_power_within_limit", "status": "pass" },
    { "check": "smb_internal_sweep_disabled", "status": "pass" },
    { "check": "magnetic_current_within_limit", "status": "pass" },
    { "check": "magnetic_ramp_within_limit", "status": "pass" },
    { "check": "magnetic_calibration_available", "status": "pass" },
    { "check": "laser_disabled_or_safe", "status": "pass" },
    { "check": "oe_passive_acquisition_only", "status": "pass" },
    { "check": "operator_approval_required", "status": "pass" },
    { "check": "expected_points_below_limit", "status": "pass" },
    { "check": "expected_runtime_below_limit", "status": "pass" },
    { "check": "no_realtime_csv", "status": "pass" },
    { "check": "no_gui_direct_hardware", "status": "pass" }
  ],
  "warnings": [],
  "errors": []
}
```

---

## 11. Dry-run Plan Model

```json
{
  "schema_version": "0.2.0",
  "kind": "dry_run_plan",
  "id": "dry_run_m5b_20260531_001",
  "resolved_recipe_id": "resolved_m5b_rf_mag_oe_system_scan",
  "summary": {
    "total_points": 9,
    "expected_frames": 45,
    "estimated_duration_s": 67.5,
    "devices": ["smb100a_main", "oe1022d_main", "maynuo_m8812_axes"],
    "outer_sweep": "mag_z_low_current_points",
    "inner_sweep": "rf_frequency_points"
  },
  "phases": [
    { "phase": "setup", "steps": ["preflight", "rf_configure", "magnetic_baseline"] },
    { "phase": "measure", "steps": ["pt_000..pt_008"] },
    { "phase": "cleanup", "steps": ["rf_output_off", "magnetic_zero", "magnetic_local"] }
  ],
  "operator_approval_required": true
}
```

**规则：**
- 不展示原始 SCPI/串口命令
- 使用高层命令类：`smb.set_frequency`, `mag.set_vector`, `oe.acquire_rall_frames`
- 危险动作（RF ON, Mag output）必须标注

---

## 12. 命令计划模型

Resolved step 中的设备动作使用高层动作名，而非原始 SCPI：

| 动作类 | 语义 | 危险等级 |
|--------|------|----------|
| `smb.set_frequency` | 设置 RF 频率 | low |
| `smb.set_power` | 设置 RF 功率 | low |
| `smb.rf_output_on` | 打开 RF 输出 | **high** |
| `smb.rf_output_off` | 关闭 RF 输出 | medium |
| `mag.set_vector` | 设置磁场矢量 | medium |
| `mag.wait_settle` | 等待磁场稳定 | low |
| `oe.configure` | 配置 OE1022D | low |
| `oe.acquire_rall_frames` | 采集 RALL 帧 | low |
| `laser.set_power` | 设置激光功率 | **high** |
| `laser.enable` | 打开激光 | **high** |

---

## 13. 可追溯性模型

每个数据点必须能回溯到：

```text
raw frame
  → index.jsonl (timestamp, step_id, frame_offset)
  → resolved_recipe.step_id
  → resolved_recipe.sweep_coordinates
  → resolved_recipe.traceability.source_recipe_id
  → resolved_recipe.traceability.required_step_hash
  → station_snapshot
  → run_id
```

---

## 14. AI 生成边界

AI 生成 recipe 时允许：
- 定义实验意图和描述
- 选择设备和固定参数
- 定义扫描轴和值列表
- 指定采集策略
- 声明安全策略开关

AI 生成 recipe 时禁止：
- 直接写入 SCPI/串口命令数组
- 覆盖安全上限数值
- 启用激光器（当前阶段）
- 声明 `safety_limits` 数值块
- 生成 `resolved_recipe` 并要求跳过 compiler

---

## 15. 当前限制（M5B-A 阶段）

| 限制 | 说明 | 后续里程碑 |
|------|------|------------|
| 磁场扫描仅支持单轴变化 | 多轴同时扫描需 coil matrix 支持 | M5B-B |
| 不支持 coil matrix 反演 | B field → current 转换固定比例 | M5B-B |
| 激光器强制禁用 | 安全策略尚未完整支持激光 | Laser-M4 |
| 不支持 block 引用 | 无 block 展开 | M5B-C |
| 不支持 profile 引用 | 固定参数直接内联 | M5B-C |
| 不支持运行时动态修改 | 执行前编译完成 | M4.2 |
| GUI 仅只读预览 | 不能从 GUI 启动系统扫描 | M4.2 |

---

## 16. 相关文件

| 文件 | 说明 |
|------|------|
| `examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json` | 示例系统扫描 recipe |
| `examples/resolved/m5b_rf_mag_oe_system_scan.resolved.json` | 展开后的 resolved recipe |
| `examples/safety/m5b_rf_mag_oe_system_scan.safety_report.json` | 安全报告示例 |
| `examples/dry_run/m5b_rf_mag_oe_system_scan.dry_run_plan.json` | 干运行计划示例 |
| `crates/odmr-recipe/src/system_scan.rs` | Rust 解析器类型 |
| `crates/odmr-compiler/src/system_scan.rs` | Rust 展开器 |
| `crates/odmr-safety/src/system_scan.rs` | Rust 安全检查 |

# Sub-PRD 04: Recipe / JSON Schema PRD v0.2

> 文件定位：Recipe / JSON Schema Sub-PRD  
> 所属项目：odor-ctl-web / ODMR Automation  
> 上级文档：00_main_prd.md v0.2、01_architecture_prd.md v0.2、02_device_registry_connection_prd.md v0.2、03_oe1022d_acquisition_prd.md v0.2  
> 当前阶段：架构冻结前的 v0.2 草案  
> 目标读者：Recipe schema 开发者、Compiler/Executor 开发者、Safety 开发者、GUI 开发者、AI/coding agent、实验设计者  

---

## 0. 文档目的

本文件定义 ODMR 自动化系统中的 **Recipe / JSON Schema** 模块。

该模块负责把人工实验配置、AI 生成实验计划、设备 profile、可复用 block、扫描逻辑、安全约束和最终执行步骤组织成一套可验证、可编译、可审计、可复现的 JSON 数据模型。

本文件不直接定义完整 JSON Schema 文件的每一行实现，而是定义 schema 的语义边界、对象关系、字段规范、校验层级、错误策略和验收标准。后续 `schemas/*.schema.json` 应以本 PRD 为依据实现。

```text
Recipe / JSON Schema 模块负责：
  station.schema.json
  device_profile.schema.json
  block.schema.json
  recipe.schema.json
  resolved_recipe.schema.json
  event.schema.json
  safety_report.schema.json
  schema versioning
  schema validation rules
  recipe 引用规则
  sweep 表达规则
  block 复用规则
  resolved_recipe 数据模型
  AI 生成 JSON 的输入/输出边界
```

```text
Recipe / JSON Schema 模块不负责：
  连接真实硬件
  执行设备命令
  展开 sweep 的具体执行代码
  实时采集线程
  raw bin 文件格式
  图表渲染
  论文级数据分析
  AI prompt 具体实现
```

---

## 1. Purpose

本模块解决的问题是：**SMB100A、OE1022D、Laser、Mag X/Y/Z 四类设备变量过多，人工 GUI 配置和 AI 自由生成都容易造成状态混乱，因此必须用结构化 JSON 把实验方案、设备配置、扫描逻辑和执行步骤严格分层。**

旧系统的问题是：

```text
GUI 参数
设备连接状态
AI agent 操作
设备命令
采集线程
实验流程
数据保存
```

容易互相交叉，导致：

```text
某个参数到底来自 GUI 还是 AI 不清楚
某次数据点对应哪个设备状态不清楚
设备命令顺序不清楚
安全限制是否被覆盖不清楚
实验能否复现不清楚
```

新系统必须把实验表达拆成以下核心对象：

```text
station
  描述当前实验站的设备清单、连接方式、全局安全上限、校准快照。

device_profile
  描述某台设备的静态或常用配置。

block
  描述可复用的实验片段，例如 RF 配置、OE1022D 配置、磁场扫描片段。

recipe
  描述实验意图、引用 profile/block、定义 sweep 和采样策略。

resolved_recipe
  Compiler 输出的最终执行计划。Executor 只接受 resolved_recipe。

run_event
  Executor 和 Acquisition Core 运行期间记录的事件。
```

核心原则：

```text
device_profile 负责设备静态/常用配置
block 负责可复用实验片段
recipe 负责组合和扫描逻辑
resolved_recipe 负责最终执行步骤
run_event 负责运行审计和数据回溯
```

---

## 2. Scope

### 2.1 本模块负责

| 对象 | 责任 |
|---|---|
| `station` | 实验站定义、设备清单、连接标识、安全边界引用 |
| `device_profile` | 单设备默认配置、常用配置、硬件能力声明 |
| `block` | 可复用实验片段，可被 recipe 引用和参数化 |
| `recipe` | 人类或 AI 编写的实验方案入口 |
| `resolved_recipe` | 编译后的顺序执行步骤，禁止再包含未展开 sweep |
| `run_event` | 运行事件格式，服务于 events.jsonl |
| `schema_version` | schema 版本、兼容策略、迁移策略 |
| `validation` | schema validation、semantic validation、safety validation 的边界 |

### 2.2 本模块不负责

```text
不负责向 SMB100A 发送 SCPI
不负责向 OE1022D 发送串口命令
不负责真实采集 RALL?
不负责磁场矩阵反演算法
不负责 raw bin 二进制格式
不负责 GUI 表单布局
不负责 AI prompt 细节
不负责最终论文拟合模型
```

---

## 3. Non-goals

第一阶段不做：

```text
复杂可视化 workflow editor
任意 JavaScript/Python 表达式嵌入 recipe
运行时动态修改 recipe
AI 在线闭环直接改 recipe 并继续执行
远程多人协作编辑 recipe
复杂插件式 schema 动态扩展系统
跨实验室统一 ontology
完整数据库后端 schema
```

第一阶段只做：

```text
明确 JSON 对象边界
明确 schema 文件结构
明确 recipe 引用和展开规则
明确 sweep 表达
明确 resolved_recipe 执行模型
明确 safety 不能被 recipe 覆盖
明确每个数据点能回溯到 step 和设备状态
```

---

## 4. Design Principles

### 4.1 Recipe 是实验意图，不是设备脚本

`recipe` 应描述实验目标和扫描逻辑，而不是直接暴露完整硬件命令序列。

允许：

```json
{
  "sweep": {
    "axis": "smb.frequency_hz",
    "start": 2820000000,
    "stop": 2920000000,
    "step": 500000
  }
}
```

不推荐：

```json
{
  "commands": [
    "FREQ 2.820GHz",
    "OUTP ON",
    "FREQ 2.8205GHz"
  ]
}
```

原因：

```text
直接命令无法统一安全检查
直接命令难以重放和统计
直接命令难以绑定数据点
直接命令容易被 AI 生成危险操作
```

### 4.2 Executor 只接受 resolved_recipe

Executor 不接受原始 `recipe`。必须先经过：

```text
schema validation
semantic validation
safety validation
compiler expansion
dry-run generation
```

才能产生：

```text
resolved_recipe.json
safety_report.json
```

Executor 启动条件：

```text
resolved_recipe schema valid
safety_report status == passed
dry-run confirmed
resource lease acquired
station_snapshot written
```

### 4.3 Safety limit 不属于 recipe

`recipe` 可以声明实验希望使用的参数，但不能修改系统安全上限。

禁止：

```json
{
  "safety_limits": {
    "smb_max_power_dbm": 20
  }
}
```

安全上限只能来自：

```text
station
safety profile
manual admin configuration
```

AI 不能生成、修改或放宽 safety limit。

### 4.4 所有数值字段必须带单位语义

字段名必须体现单位，避免混淆。

推荐：

```text
frequency_hz
power_dbm
dwell_ms
settle_ms
voltage_v
current_a
magnetic_field_t
phase_deg
time_constant_s
```

禁止模糊字段：

```text
freq
power
level
time
value
mag
```

除非字段是通用对象，并且内部显式写明 unit。

### 4.5 可读性优先于过度抽象

第一阶段优先让实验者、coding agent、审稿者能读懂。不要把 recipe 设计成过度 DSL。

---

## 5. Inputs

### 5.1 输入文件

| 输入 | 说明 |
|---|---|
| `station.json` | 当前实验站设备、连接、capability、安全引用 |
| `device_profiles/*.json` | 设备默认配置和常用配置 |
| `blocks/*.json` | 可复用实验片段 |
| `recipes/*.recipe.json` | 实验方案 |
| `schemas/*.schema.json` | JSON Schema 文件 |
| `safety_limits.json` | 安全上限，recipe 不可覆盖 |
| `calibration_snapshot.json` | coil matrix、zero offset、设备校准信息 |

### 5.2 输入来源

```text
人类手写 JSON
GUI 表单生成 JSON
AI agent 生成 JSON
已有 recipe 复制修改
block 库组合
```

### 5.3 输入限制

所有输入必须是纯数据：

```text
不允许脚本
不允许 eval
不允许 shell command
不允许 Python expression
不允许 JavaScript expression
不允许直接硬件命令数组作为主执行模型
```

---

## 6. Outputs

### 6.1 输出文件

| 输出 | 生成者 | 消费者 | 说明 |
|---|---|---|---|
| `resolved_recipe.json` | Compiler | Executor | 最终执行步骤 |
| `safety_report.json` | Compiler / Safety | GUI / Executor | 安全检查结果 |
| `dry_run_plan.json` | Compiler | GUI / 人类 | 可读的执行预览 |
| `station_snapshot.json` | Executor | Data Logging | 运行时设备状态快照 |
| `events.jsonl` | Executor | Data Logging / Analysis | 运行事件 |
| `index.jsonl` | Acquisition Core | Data Logging | raw frame 索引 |
| `parsed.parquet` | Post-run parser | Analysis | 解析后数据 |
| `export.csv` | Exporter | 人类/论文 | 导出格式，不是实时格式 |

### 6.2 输出要求

每个 `resolved_recipe.step` 必须能关联：

```text
recipe_id
block_id if any
profile_id if any
step_id
sweep coordinates
expected device state
expected acquisition window
safety check result
```

每个数据点最终必须能回溯到：

```text
run_id
step_id
resolved_recipe hash
station_snapshot hash
device state hash
```

---

## 7. Runtime Model

### 7.1 编译前

```text
Human / GUI / AI
  → recipe.json
  → schema validation
  → semantic validation
  → safety pre-check
```

### 7.2 编译

```text
Recipe Compiler
  reads station.json
  reads device_profile.json
  reads block.json
  reads recipe.json
  expands block references
  expands sweep
  inserts dwell / settle / acquisition window
  computes expected duration
  generates resolved_recipe.json
  generates safety_report.json
  generates dry_run_plan.json
```

### 7.3 执行

```text
Executor
  reads resolved_recipe.json
  reads safety_report.json
  checks safety_report == passed
  acquires resource leases
  writes station_snapshot.json
  starts acquisition core if required
  executes steps sequentially
  writes events.jsonl
```

### 7.4 数据回溯

```text
raw frame
  → index.jsonl
  → step_id
  → resolved_recipe.json
  → station_snapshot.json
  → parsed.parquet
```

---

## 8. Data Model

## 8.1 Common Fields

所有顶层对象必须包含：

```json
{
  "schema_version": "0.2.0",
  "kind": "recipe",
  "id": "odmr_basic_sweep_v1",
  "name": "ODMR basic frequency sweep",
  "created_by": "human_or_ai",
  "created_at": "2026-05-28T00:00:00Z",
  "description": "..."
}
```

### 8.1.1 `schema_version`

格式：

```text
MAJOR.MINOR.PATCH
```

规则：

```text
MAJOR 改变：不兼容变更
MINOR 改变：向后兼容新增字段
PATCH 改变：文档、校验说明、非结构修正
```

### 8.1.2 `kind`

允许值：

```text
station
device_profile
block
recipe
resolved_recipe
run_event
safety_report
```

### 8.1.3 `id`

规则：

```text
小写字母、数字、下划线、短横线
不得包含空格
不得包含中文
不得包含路径分隔符
稳定，不应频繁改动
```

示例：

```text
station_lab_nv_room_01
smb100a_profile_fm_500hz
odmr_basic_frequency_sweep
```

---

## 8.2 `station`

### 8.2.1 Purpose

`station` 描述当前实验站，包括设备清单、连接方式、全局安全配置引用、校准信息和默认 profile。

### 8.2.2 Example

```json
{
  "schema_version": "0.2.0",
  "kind": "station",
  "id": "station_nv_lab_01",
  "name": "NV ODMR Station 01",
  "devices": [
    {
      "device_id": "smb100a_01",
      "type": "smb100a",
      "transport": {
        "kind": "tcp_scpi",
        "host": "192.168.0.20",
        "port": 5025
      },
      "profile_id": "smb100a_default_fm_500hz",
      "required": true
    },
    {
      "device_id": "oe1022d_01",
      "type": "oe1022d",
      "transport": {
        "kind": "serial",
        "port": "COM3",
        "baud_rate": 115200
      },
      "profile_id": "oe1022d_chb_default",
      "required": true
    },
    {
      "device_id": "laser_01",
      "type": "laser",
      "transport": {
        "kind": "serial",
        "port": "COM4"
      },
      "required": false
    },
    {
      "device_id": "mag_xyz_01",
      "type": "magnet_xyz",
      "transport": {
        "kind": "serial_group",
        "ports": ["COM5", "COM6", "COM7"]
      },
      "profile_id": "mag_xyz_default",
      "required": true
    }
  ],
  "safety_profile_id": "safety_nv_station_default",
  "calibration_snapshot_id": "calibration_2026_05_28"
}
```

### 8.2.3 Required Fields

```text
schema_version
kind
id
devices
safety_profile_id
```

### 8.2.4 Station Validation Rules

```text
device_id 必须唯一
device type 必须来自受支持设备类型枚举
transport.kind 必须和 device type 兼容
required == true 的设备在执行前必须 connected
station 不包含实验流程
station 不包含 sweep
station 不包含 AI prompt
```

---

## 8.3 `device_profile`

### 8.3.1 Purpose

`device_profile` 描述设备默认配置和常用配置。它不是实验扫描逻辑，只是设备状态模板。

### 8.3.2 SMB100A Profile Example

```json
{
  "schema_version": "0.2.0",
  "kind": "device_profile",
  "id": "smb100a_default_fm_500hz",
  "device_type": "smb100a",
  "settings": {
    "rf": {
      "mode": "cw",
      "frequency_hz": 2882000000,
      "power_dbm": -15.0,
      "output_enabled": false,
      "alc": "auto"
    },
    "lf": {
      "output_enabled": true,
      "frequency_hz": 500.0,
      "shape": "square",
      "voltage_v": 0.137
    },
    "fm": {
      "enabled": true,
      "source": "internal_lf",
      "deviation_hz": 4000000
    },
    "modulation_global_enabled": false
  }
}
```

### 8.3.3 OE1022D Profile Example

```json
{
  "schema_version": "0.2.0",
  "kind": "device_profile",
  "id": "oe1022d_chb_default",
  "device_type": "oe1022d",
  "settings": {
    "channel": "B",
    "reference": {
      "source": "external",
      "trigger": "ttl_rising_edge",
      "phase_deg": 0.0
    },
    "input_filter": {
      "input_source": "voltage_single_ended",
      "coupling": "ac",
      "grounding": "ground",
      "notch_filter": "off"
    },
    "gain_time_constant": {
      "sensitivity_v": 0.1,
      "time_constant_s": 0.3,
      "filter_slope_db_per_oct": 12,
      "dynamic_reserve": "normal"
    },
    "sample": {
      "read_command": "RALL?",
      "sample_interval_ms": 50,
      "channels": ["x", "y", "r", "theta", "freq", "noise"]
    }
  }
}
```

### 8.3.4 Magnetic Profile Example

```json
{
  "schema_version": "0.2.0",
  "kind": "device_profile",
  "id": "mag_xyz_default",
  "device_type": "magnet_xyz",
  "settings": {
    "coordinate_system": "cartesian",
    "coil_matrix_id": "coil_matrix_2026_05_28",
    "zero_offset_a": {
      "x": 0.0,
      "y": 0.0,
      "z": 0.0
    },
    "ramp_limit_a_per_s": 0.05,
    "settle_ms": 500
  }
}
```

### 8.3.5 Profile Validation Rules

```text
profile 不能声明安全上限
profile 不能发出执行动作
profile 可以声明 output_enabled 的期望初态，但执行前仍需 safety 检查
profile 中所有数值必须带单位字段名
profile 只能被 station、block、recipe 引用
```

---

## 8.4 `block`

### 8.4.1 Purpose

`block` 是可复用实验片段。它可以表达：

```text
SMB100A RF/FM 配置片段
OE1022D 锁相配置片段
磁场扫描片段
激光器稳定等待片段
采集窗口片段
```

block 不直接运行。block 必须被 recipe 引用，然后由 Compiler 展开。

### 8.4.2 Block Example: SMB FM Setup

```json
{
  "schema_version": "0.2.0",
  "kind": "block",
  "id": "block_smb_fm_500hz_4mhz",
  "block_type": "device_setup",
  "device_type": "smb100a",
  "parameters": {
    "rf_frequency_hz": 2882000000,
    "rf_power_dbm": -15.0,
    "lf_frequency_hz": 500.0,
    "lf_shape": "square",
    "lf_voltage_v": 0.137,
    "fm_deviation_hz": 4000000,
    "fm_enabled": true,
    "modulation_global_enabled": true,
    "rf_output_enabled": true
  }
}
```

### 8.4.3 Block Example: Magnetic Grid Sweep

```json
{
  "schema_version": "0.2.0",
  "kind": "block",
  "id": "block_bz_linear_sweep_small",
  "block_type": "magnetic_sweep",
  "coordinate_system": "cartesian",
  "sweep": {
    "axis": "b_z_t",
    "start": -0.001,
    "stop": 0.001,
    "step": 0.0001,
    "fixed": {
      "b_x_t": 0.0,
      "b_y_t": 0.0
    }
  },
  "settle_ms": 500
}
```

### 8.4.4 Block Validation Rules

```text
block.id 必须唯一
block_type 必须来自枚举
block 可以带参数，但不允许代码表达式
block 可以被 recipe 多次引用
block 展开后必须保留 source_block_id
block 不能直接覆盖 safety limit
```

---

## 8.5 `recipe`

### 8.5.1 Purpose

`recipe` 是实验者或 AI 生成的实验方案入口。它表达：

```text
实验目标
station 引用
profile 引用
block 组合
扫描轴
采样窗口
平均策略
数据标签
```

### 8.5.2 Basic Recipe Example

```json
{
  "schema_version": "0.2.0",
  "kind": "recipe",
  "id": "odmr_basic_frequency_sweep",
  "name": "ODMR basic frequency sweep",
  "station_id": "station_nv_lab_01",
  "intent": {
    "experiment_type": "cw_odmr",
    "description": "Sweep SMB100A RF frequency around 2.87 GHz and acquire OE1022D Ch-B X signal."
  },
  "profiles": [
    "smb100a_default_fm_500hz",
    "oe1022d_chb_default",
    "mag_xyz_default"
  ],
  "blocks": [
    "block_smb_fm_500hz_4mhz"
  ],
  "sweeps": [
    {
      "sweep_id": "rf_frequency_sweep",
      "axis": "smb100a.rf.frequency_hz",
      "start": 2820000000,
      "stop": 2920000000,
      "step": 500000,
      "order": "ascending"
    }
  ],
  "acquisition": {
    "device_id": "oe1022d_01",
    "channel": "B",
    "readout": ["x", "y", "r", "theta", "freq", "noise"],
    "pre_discard_ms": 300,
    "window_ms": 500,
    "average": {
      "mode": "mean",
      "repeat": 1
    }
  },
  "timing": {
    "default_dwell_ms": 500,
    "default_settle_ms": 500
  },
  "metadata": {
    "sample_id": "diamond_nv_sample_01",
    "operator": "manual",
    "notes": "Initial v0.2 schema example."
  }
}
```

### 8.5.3 Nested Sweep Example

```json
{
  "sweeps": [
    {
      "sweep_id": "bz_sweep",
      "axis": "mag.b_z_t",
      "start": -0.001,
      "stop": 0.001,
      "step": 0.0001,
      "settle_ms": 500
    },
    {
      "sweep_id": "rf_sweep",
      "axis": "smb100a.rf.frequency_hz",
      "start": 2820000000,
      "stop": 2920000000,
      "step": 500000,
      "dwell_ms": 500
    }
  ],
  "sweep_order": ["bz_sweep", "rf_sweep"]
}
```

语义：

```text
外层先扫 Bz
每一个 Bz 点下扫完整 RF frequency
每一个最终点采集 OE1022D window
```

### 8.5.4 Recipe Validation Rules

Schema validation 检查：

```text
字段类型正确
必填字段存在
枚举值合法
数值范围基础合法
引用字段格式正确
```

Semantic validation 检查：

```text
station_id 存在
profile_id 存在
block_id 存在
sweep axis 能映射到设备参数
sweep step 方向和 start/stop 一致
acquisition window 和 dwell/settle 不冲突
同一个设备没有互斥状态冲突
```

Safety validation 检查：

```text
SMB power 不超过 station safety limit
SMB frequency 在设备 capability 范围内
FM deviation 不超过安全或设备范围
laser power 不超过安全上限
mag current 不超过安全上限
mag ramp rate 不超过安全上限
OE1022D overload handling 已定义
```

---

## 8.6 `resolved_recipe`

### 8.6.1 Purpose

`resolved_recipe` 是 Compiler 生成的最终执行计划。它必须是顺序、显式、不可再解释的结构。

Executor 只能执行 `resolved_recipe`，不能执行 `recipe`。

### 8.6.2 Example

```json
{
  "schema_version": "0.2.0",
  "kind": "resolved_recipe",
  "id": "resolved_odmr_basic_frequency_sweep_20260528_001",
  "source_recipe_id": "odmr_basic_frequency_sweep",
  "source_recipe_hash": "sha256:...",
  "station_id": "station_nv_lab_01",
  "estimated_duration_s": 402.5,
  "safety_report_id": "safety_report_20260528_001",
  "steps": [
    {
      "step_id": "step_000001",
      "source_block_id": "block_smb_fm_500hz_4mhz",
      "phase": "setup",
      "device_actions": [
        {
          "device_id": "smb100a_01",
          "action": "set_rf_frequency",
          "params": {
            "frequency_hz": 2820000000
          }
        },
        {
          "device_id": "smb100a_01",
          "action": "set_rf_power",
          "params": {
            "power_dbm": -15.0
          }
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
        "pre_discard_ms": 300,
        "window_ms": 500,
        "readout": ["x", "y", "r", "theta", "freq", "noise"]
      },
      "sweep_coordinates": {
        "smb100a.rf.frequency_hz": 2820000000
      }
    }
  ]
}
```

### 8.6.3 Resolved Step Rules

每个 step 必须包含：

```text
step_id
phase
device_actions
expected_state
timing
acquisition if needed
sweep_coordinates if generated from sweep
source_block_id if generated from block
```

禁止包含：

```text
未展开 sweep
未解析 block reference
自由文本命令
AI prompt
安全上限修改
动态代码表达式
```

### 8.6.4 Step ID Rules

格式：

```text
step_000001
step_000002
...
```

要求：

```text
全局唯一
稳定排序
不依赖数组下标进行数据回溯
写入 events.jsonl
写入 index.jsonl
写入 parsed.parquet
```

---

## 8.7 `run_event`

### 8.7.1 Purpose

`run_event` 是事件日志对象，用于记录执行过程、错误、警告、设备状态变化和安全事件。

### 8.7.2 Example

```json
{
  "schema_version": "0.2.0",
  "kind": "run_event",
  "run_id": "run_20260528_001",
  "event_id": "evt_000012",
  "timestamp_wall": "2026-05-28T12:00:01.500Z",
  "timestamp_monotonic_ns": 123456789000,
  "level": "info",
  "event_type": "step_started",
  "step_id": "step_000001",
  "device_id": "smb100a_01",
  "message": "Step started.",
  "data": {
    "rf_frequency_hz": 2820000000
  }
}
```

### 8.7.3 Event Types

第一阶段必须支持：

```text
run_created
run_started
run_stopped
run_completed
run_failed
step_started
step_completed
step_failed
device_connected
device_disconnected
device_command_sent
device_command_completed
device_command_failed
acquisition_started
acquisition_stopped
frame_dropped
safety_check_passed
safety_check_failed
emergency_stop
manual_override
warning
error
```

---

## 9. API / Interface

## 9.1 Schema Directory

目标目录：

```text
docs/
  schemas/
    station.schema.json
    device_profile.schema.json
    block.schema.json
    recipe.schema.json
    resolved_recipe.schema.json
    event.schema.json
    safety_report.schema.json
```

## 9.2 Rust Interfaces

建议 Rust crate：

```text
recipe-schema
recipe-compiler
safety-core
executor-core
```

核心类型：

```rust
struct Station;
struct DeviceProfile;
struct Block;
struct Recipe;
struct ResolvedRecipe;
struct ResolvedStep;
struct RunEvent;
struct SafetyReport;
```

核心函数：

```rust
validate_schema(json: &Value, schema_kind: SchemaKind) -> ValidationResult
validate_semantics(recipe: &Recipe, station: &Station, blocks: &[Block]) -> ValidationResult
compile_recipe(recipe: &Recipe, station: &Station, profiles: &[DeviceProfile], blocks: &[Block]) -> CompileResult<ResolvedRecipe>
generate_dry_run(resolved: &ResolvedRecipe) -> DryRunPlan
```

## 9.3 GUI Interfaces

GUI 只允许调用：

```text
load_recipe
validate_recipe
compile_recipe
display_dry_run
display_safety_report
submit_resolved_recipe_to_executor
```

GUI 不允许：

```text
直接修改 resolved_recipe 后执行
绕过 safety_report
直接向设备发命令
在前端展开 sweep 后执行
```

## 9.4 AI Agent Interfaces

AI agent 只允许输出：

```text
recipe.json draft
block.json draft
device_profile.json draft suggestion
schema validation fix suggestion
```

AI agent 不允许：

```text
修改 station safety limit
修改 emergency stop 规则
直接生成 resolved_recipe 并要求执行
直接生成设备命令并要求发送
直接访问硬件
```

---

## 10. Safety Rules

### 10.1 General Safety Rules

```text
recipe 不能覆盖 safety limit
block 不能覆盖 safety limit
profile 不能覆盖 safety limit
AI 不能覆盖 safety limit
resolved_recipe 必须带 safety_report_id
safety_report failed 时 Executor 拒绝执行
```

### 10.2 SMB100A Safety

必须检查：

```text
rf_power_dbm <= smb_max_power_dbm
rf_frequency_hz within capability
fm_deviation_hz within allowed range
rf_output_enabled 的步骤必须 dry-run 可见
modulation_global_enabled 的步骤必须 dry-run 可见
```

### 10.3 OE1022D Safety

必须检查：

```text
channel 配置明确
input source 明确
sensitivity 合理
filter/time constant 合理
overload handling 已定义
采集命令不会和状态查询互相打断
```

### 10.4 Laser Safety

必须检查：

```text
laser_power_mw <= laser_max_power_mw
laser_enable 步骤必须 dry-run 可见
laser warmup / settle 明确
safe shutdown 明确
```

### 10.5 Magnetic Safety

必须检查：

```text
coil current <= mag_max_current
ramp rate <= mag_max_ramp_rate
B vector 转 coil current 后仍安全
zero offset 已应用
settle_ms 明确
```

---

## 11. Error Handling

### 11.1 Schema Validation Errors

示例：

```text
missing required field: station_id
invalid enum: lf_shape = "sq"
invalid type: frequency_hz = "2.87GHz"
```

处理：

```text
GUI 高亮字段
AI 可生成修复建议
Compiler 不继续
Executor 不可启动
```

### 11.2 Semantic Validation Errors

示例：

```text
unknown block_id
unknown profile_id
sweep axis cannot map to device parameter
nested sweep order references missing sweep_id
```

处理：

```text
Compiler 返回 structured error
dry-run 不生成
Executor 不可启动
```

### 11.3 Safety Validation Errors

示例：

```text
SMB power exceeds limit
magnetic ramp rate exceeds limit
laser power exceeds limit
OE1022D overload policy missing
```

处理：

```text
生成 safety_report.json
status = failed
GUI 展示拒绝原因
Executor 拒绝执行
```

### 11.4 Compile Errors

示例：

```text
sweep point count too large
estimated duration exceeds allowed run duration
step count exceeds executor limit
block parameter conflict
```

处理：

```text
Compiler 失败
不生成 resolved_recipe
可生成 partial diagnostic
```

---

## 12. Test Harness

### 12.1 Required Tests

```text
schema validation tests
invalid schema rejection tests
semantic validation tests
safety rejection tests
block expansion tests
sweep expansion tests
resolved_recipe snapshot tests
dry-run output tests
AI-generated recipe validation tests
backward compatibility tests
```

### 12.2 Test Data

目录建议：

```text
tests/
  fixtures/
    valid/
      station.valid.json
      smb100a_profile.valid.json
      oe1022d_profile.valid.json
      block_smb_fm.valid.json
      recipe_basic_odmr.valid.json
    invalid/
      recipe_missing_station.invalid.json
      recipe_bad_axis.invalid.json
      recipe_unsafe_power.invalid.json
      recipe_bad_sweep.invalid.json
```

### 12.3 Snapshot Tests

对于固定 recipe，Compiler 输出必须稳定：

```text
same input recipe
same station
same profiles
same blocks
→ same resolved_recipe hash
```

允许变化：

```text
created_at
generated_at
run_id
```

不允许变化：

```text
step order
step_id
sweep_coordinates
expected_state
safety decisions
```

---

## 13. Acceptance Criteria

v0.2 验收标准：

```text
能定义 station.schema.json 的字段草案
能定义 device_profile.schema.json 的字段草案
能定义 block.schema.json 的字段草案
能定义 recipe.schema.json 的字段草案
能定义 resolved_recipe.schema.json 的字段草案
能定义 event.schema.json 的字段草案
能写出 odmr_basic_sweep.recipe.json 示例
能写出 odmr_bz_sweep.recipe.json 示例
能把 recipe 编译成 resolved_recipe 示例
能生成 dry-run_plan 示例
能生成 safety_report 示例
能拒绝不安全 recipe
能拒绝 AI 生成的危险字段
```

v0.3 验收标准：

```text
Rust struct 与 JSON Schema 一致
schema validation 自动化测试通过
recipe compiler MVP 通过 snapshot tests
Executor 能读取 resolved_recipe 并 dry-run
GUI 能加载 recipe 并展示 validation error
AI agent 能生成 recipe draft 但不能绕过 validation
```

v1.0 验收标准：

```text
真实设备实验 run 的每个数据点都能回溯到 step_id
resolved_recipe 与 station_snapshot 被写入 run directory
events.jsonl 完整记录设备动作
unsafe recipe 永远不能执行
schema migration 有基本策略
```

---

## 14. Agent Constraints

### 14.1 Recipe Schema Agent

允许：

```text
修改 schemas/*.schema.json
修改 examples/*.json
修改本 PRD 的字段说明
新增 validation tests
```

禁止：

```text
修改 Executor 真实硬件执行逻辑
修改 OE1022D 采集线程
修改 SMB100A driver
修改 safety limit 默认值
绕过 safety validation
```

### 14.2 Recipe Authoring Agent

允许：

```text
根据实验目标生成 recipe draft
根据 validation error 修复 recipe
生成 block draft
解释 recipe 含义
生成 dry-run 说明文本
```

禁止：

```text
生成直接设备命令作为执行入口
生成 resolved_recipe 并要求跳过 Compiler
修改 safety limit
修改 station 连接信息
声称 recipe 已经安全，除非 safety_report passed
```

### 14.3 Review Checklist

每次 agent 修改 recipe/schema 后必须检查：

```text
是否有 schema_version
是否有 kind
是否所有数值字段带单位语义
是否没有脚本表达式
是否没有直接硬件命令数组
是否没有 safety limit 覆盖
是否能通过 schema validation
是否能通过 semantic validation
是否能生成 dry-run
是否能生成 safety_report
```

---

## 15. Open Questions

待后续 PRD 或 ADR 决定：

```text
是否采用 JSON Schema Draft 2020-12 作为长期版本
Rust 是否使用 schemars 反向生成 schema，还是手写 schema
recipe 中是否允许矩阵式 sweep 和 zip sweep 两种模式
超大 sweep 是否允许分块 compile
resolved_recipe 是否允许压缩表示，还是必须完全展开
block parameter override 语义是否需要更复杂的继承规则
```

当前 v0.2 建议：

```text
优先完全展开 resolved_recipe
优先手写 schema + Rust struct 对照测试
优先简单 nested sweep
暂不支持脚本表达式和复杂继承
```

---

## 16. References

- JSON Schema Draft 2020-12 official specification: https://json-schema.org/draft/2020-12
- JSON Schema reference documentation: https://json-schema.org/understanding-json-schema/reference
- Project PRD dependency: `00_main_prd.md`, `01_architecture_prd.md`, `02_device_registry_connection_prd.md`, `03_oe1022d_acquisition_prd.md`

---

## 17. Summary

`04 Recipe / JSON Schema` 是整个系统的实验表达核心。它决定：

```text
AI 能生成什么
GUI 能编辑什么
Compiler 能展开什么
Executor 能执行什么
Safety 能拒绝什么
Data Logging 能回溯什么
```

本模块的基本设计结论是：

```text
recipe 表达实验意图
block 表达可复用片段
device_profile 表达设备默认状态
station 表达实验站和安全引用
resolved_recipe 表达最终执行计划
run_event 表达运行审计
```

所有硬件执行必须从 `resolved_recipe` 开始，所有实验数据必须能回溯到 `step_id`。这条边界不能被 GUI、AI agent 或人工临时操作绕过。

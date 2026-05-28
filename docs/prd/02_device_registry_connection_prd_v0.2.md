# Sub-PRD 02: Device Registry & Connection PRD v0.2

> 文件定位：设备注册表与连接管理 Sub-PRD  
> 所属项目：odor-ctl-web / ODMR Automation  
> 上级文档：00_main_prd.md v0.2、01_architecture_prd.md v0.2  
> 当前阶段：架构冻结前的 v0.2 草案  
> 目标读者：Rust 后端开发者、设备驱动开发者、Executor 开发者、GUI 开发者、AI/coding agent  

---

## 0. 文档目的

本文件定义 ODMR 自动化系统中的 **Device Registry & Connection** 模块。

该模块负责把实验站中的真实设备、mock 设备、连接参数、设备身份、设备能力、连接状态、资源租约统一管理起来，防止 GUI、executor、采集线程、AI agent 或测试脚本各自直接连接硬件。

本模块是系统中所有设备访问的入口之一，但它本身不是实验执行器，也不是高速采集模块。

```text
Device Registry 负责：
  设备是谁
  设备在哪里
  用什么 transport 连接
  IDN 是否匹配
  当前是否在线
  当前由谁持有资源租约
  设备具备哪些 capability
  如何安全连接与安全断开
```

```text
Device Registry 不负责：
  实验流程
  recipe 展开
  高频采集
  图表刷新
  AI 生成
  谱线拟合
  数据库训练
```

---

## 1. Purpose

本模块解决的问题是：**在多设备、多线程、多 agent 开发环境下，建立唯一、可审计、可测试的设备注册和连接层。**

当前实验系统至少涉及：

```text
1. SMB100A RF / microwave signal generator
2. OE1022D DSP lock-in amplifier
3. Laser controller
4. Mag X axis current source / magnetic axis controller
5. Mag Y axis current source / magnetic axis controller
6. Mag Z axis current source / magnetic axis controller
```

如果没有统一 Device Registry，系统会退化成：

```text
GUI 直接连串口
Executor 直接连 TCP socket
采集线程直接连 OE1022D
测试脚本直接连假设备
Python notebook 也可能直接打开同一个端口
```

这会导致：

```text
1. 同一设备被多个模块同时访问
2. 串口 / socket / VISA session 被并发占用
3. 设备连接参数散落在代码和 GUI 配置中
4. 设备身份无法验证，容易连错设备
5. mock 和真实设备行为不一致
6. 断连、重连、超时和安全关闭逻辑重复实现
7. 实验数据无法准确记录当时使用的设备与连接状态
```

Device Registry 的目标是建立一个统一边界：

```text
所有模块先向 Device Manager 查询设备；
所有真实连接由 Device Manager 创建、持有、释放；
所有硬件访问必须经过资源租约；
所有实验 run 必须保存 station_snapshot.json。
```

---

## 2. Scope

本 PRD 负责定义：

```text
1. station / device_profile / device_binding 的职责
2. device_id 命名规范
3. device kind 与 device role
4. transport 类型
5. port / visa_address / tcp address 等连接字段
6. IDN 查询与身份匹配规则
7. connection status 状态机
8. capability 描述方式
9. resource lease 模型
10. safe connect / safe disconnect 行为
11. mock / real / dry-run 三种连接模式
12. GUI 与 Executor 需要调用的连接 API
13. 连接事件日志格式
14. 连接层测试 harness
15. AI agent 在本模块中的允许与禁止行为
```

---

## 3. Non-goals

本模块第一阶段明确不负责：

```text
1. 实验 recipe 展开
2. 实验步骤调度
3. OE1022D RALL? 高频采集循环
4. RALL? frame 解析
5. SMB100A 完整 SCPI 指令编排
6. 磁场路径规划
7. 激光功率闭环控制
8. 实时图表数据流
9. raw bin / parquet 文件格式
10. AI prompt 设计
11. GUI 页面布局
12. 论文级数据分析
13. 多用户权限系统
14. 远程服务器部署
```

特别说明：

```text
Device Registry 可以创建 OE1022D 连接，
但不能在 Registry 内部执行 RALL? 高频采集循环。

Device Registry 可以检查 SMB100A IDN，
但不能在 Registry 内部执行 ODMR 扫频逻辑。

Device Registry 可以保存 Mag X/Y/Z 的端口和能力，
但不能在 Registry 内部展开磁场 sweep。
```

---

## 4. Design Principles

### 4.1 单一连接所有权

每个真实设备在运行时只能由 Device Manager 持有一个底层连接对象。

```text
正确：
GUI -> Tauri command -> Device Manager -> device connection
Executor -> Device Manager lease -> device driver
Acquisition Core -> Device Manager lease -> OE1022D connection
```

```text
禁止：
GUI -> serial port
AI agent -> TCP socket
Python notebook -> running device port
Executor -> independent VISA/session outside Device Manager
```

---

### 4.2 配置状态和运行状态分离

Device Registry 中要区分：

```text
静态配置：station.json / device_profile.json
运行状态：connection_status / lease / last_error / last_seen
实验快照：station_snapshot.json
```

静态配置可以被编辑和版本控制。运行状态不应直接写回 profile，避免把临时断连、错误、占用信息污染长期配置。

---

### 4.3 设备身份优先于端口名称

串口号、IP 地址、VISA address 都可能变化。Device Registry 不能只依赖端口名判断设备。

连接时必须尽量执行：

```text
1. open transport
2. query identity
3. match expected_idn / idn_regex / device fingerprint
4. update runtime status
5. emit connection event
```

如果设备不支持 IDN 查询，则必须定义替代验证方式：

```text
1. handshake command
2. firmware/version query
3. known response pattern
4. manual_verified flag
5. explicit unsafe_unverified permission in development only
```

---

### 4.4 Mock 与真实设备同接口

Device Registry 必须允许同一套 API 在以下模式运行：

```text
real      连接真实硬件
mock      使用 fake device / fake transport
dry_run   不打开真实端口，只校验配置和计划连接动作
replay    从记录文件模拟历史设备状态
```

第一阶段必须保证：

```text
没有真实设备时，系统仍能启动；
没有真实设备时，GUI 仍能显示设备列表；
没有真实设备时，recipe validation / dry-run / mock-run 仍能跑；
有真实设备时，必须通过同一个 Device Manager 访问。
```

---

### 4.5 连接层不做实验语义

Device Registry 不理解 ODMR 谱线、不理解磁场扫描、不理解机器学习目标。

它只理解：

```text
device
transport
identity
status
capability
lease
safe state
```

实验语义由 Recipe Compiler、Executor、Magnetic Planner、Acquisition Core、Data Logging 模块处理。

---

## 5. Device Inventory

### 5.1 第一阶段设备列表

| Device ID | Kind | Role | Required | Primary transport | Notes |
|---|---|---|---:|---|---|
| `smb100a.main` | `smb100a` | RF microwave source | yes | `tcp_scpi` | 第一阶段优先使用 LAN socket SCPI；Python/RsInstrument 可作为人工验证路径 |
| `oe1022d.main` | `oe1022d` | lock-in amplifier | yes | `serial` | Device Registry 管连接；高频采集由 OE1022D Acquisition Core 执行 |
| `laser.main` | `laser` | laser controller | yes | `serial` | 第一阶段只定义连接与基础状态；具体协议下沉到 laser driver |
| `mag.x` | `mag_axis` | magnetic X axis | yes | `serial` / `tcp` | 负责 X 轴电流源或磁场控制器连接 |
| `mag.y` | `mag_axis` | magnetic Y axis | yes | `serial` / `tcp` | 负责 Y 轴电流源或磁场控制器连接 |
| `mag.z` | `mag_axis` | magnetic Z axis | yes | `serial` / `tcp` | 负责 Z 轴电流源或磁场控制器连接 |

---

### 5.2 Device Kind

```text
smb100a
  RF / microwave signal generator

oe1022d
  DSP lock-in amplifier

laser
  laser source or laser controller

mag_axis
  single magnetic axis current source / driver

mag_controller
  optional future integrated 3-axis magnetic controller

mock
  test-only fake device
```

---

### 5.3 Device Role

`kind` 描述设备类型，`role` 描述设备在当前实验站中的用途。

示例：

```text
kind = smb100a
role = rf_source

a future second SMB100A could be:
kind = smb100a
role = calibration_rf_source
```

第一阶段推荐 role：

```text
rf_source
lockin
laser_source
mag_x
mag_y
mag_z
```

---

## 6. Inputs

### 6.1 配置输入文件

Device Registry 主要读取：

```text
config/station.json
config/device_profiles/*.json
config/transport_profiles/*.json      optional
config/safety_limits.json             read-only reference
```

实验 run 开始时生成：

```text
runs/<run_id>/metadata/station_snapshot.json
runs/<run_id>/events/events.jsonl
```

---

### 6.2 station.json

`station.json` 描述当前实验站有哪些设备、绑定哪些 profile、默认连接模式是什么。

示例：

```json
{
  "schema_version": "0.2",
  "station_id": "nv_odmr_station_01",
  "station_name": "NV ODMR Main Bench",
  "default_mode": "real",
  "devices": [
    {
      "device_id": "smb100a.main",
      "kind": "smb100a",
      "role": "rf_source",
      "profile": "profiles/smb100a_main.profile.json",
      "enabled": true,
      "required": true
    },
    {
      "device_id": "oe1022d.main",
      "kind": "oe1022d",
      "role": "lockin",
      "profile": "profiles/oe1022d_main.profile.json",
      "enabled": true,
      "required": true
    },
    {
      "device_id": "laser.main",
      "kind": "laser",
      "role": "laser_source",
      "profile": "profiles/laser_main.profile.json",
      "enabled": true,
      "required": true
    },
    {
      "device_id": "mag.x",
      "kind": "mag_axis",
      "role": "mag_x",
      "profile": "profiles/mag_x.profile.json",
      "enabled": true,
      "required": true
    },
    {
      "device_id": "mag.y",
      "kind": "mag_axis",
      "role": "mag_y",
      "profile": "profiles/mag_y.profile.json",
      "enabled": true,
      "required": true
    },
    {
      "device_id": "mag.z",
      "kind": "mag_axis",
      "role": "mag_z",
      "profile": "profiles/mag_z.profile.json",
      "enabled": true,
      "required": true
    }
  ]
}
```

---

### 6.3 device_profile.json

`device_profile` 描述单台设备的连接方式、身份匹配、能力、启动安全状态。

#### 6.3.1 SMB100A profile 示例

```json
{
  "schema_version": "0.2",
  "device_id": "smb100a.main",
  "kind": "smb100a",
  "display_name": "R&S SMB100A Main RF Source",
  "transport": {
    "type": "tcp_scpi",
    "host": "192.168.0.20",
    "port": 5025,
    "timeout_ms": 2000,
    "line_ending": "\n"
  },
  "identity": {
    "query": "*IDN?",
    "expected_contains": ["ROHDE", "SCHWARZ", "SMB100A"],
    "required": true
  },
  "capabilities": {
    "rf_frequency_hz": { "min": 100000.0, "max": 12750000000.0 },
    "rf_power_dbm": { "min": -120.0, "max": 20.0 },
    "supports_fm": true,
    "supports_lf_output": true,
    "supports_frequency_sweep": true
  },
  "safe_connect": {
    "commands": [
      "OUTP OFF",
      "MOD:STAT OFF"
    ]
  },
  "safe_disconnect": {
    "commands": [
      "OUTP OFF",
      "MOD:STAT OFF"
    ],
    "close_transport": true
  }
}
```

说明：

```text
SMB100A 第一阶段推荐 tcp_scpi。
VISA address 可以作为可选字段保留，但不作为 v0.2 的首选路径。
```

#### 6.3.2 OE1022D profile 示例

```json
{
  "schema_version": "0.2",
  "device_id": "oe1022d.main",
  "kind": "oe1022d",
  "display_name": "SSI OE1022D Lock-in Amplifier",
  "transport": {
    "type": "serial",
    "port": "COM3",
    "baud_rate": 115200,
    "data_bits": 8,
    "stop_bits": 1,
    "parity": "none",
    "timeout_ms": 1000,
    "line_ending": "\r"
  },
  "identity": {
    "query": "*IDN?",
    "expected_contains": ["OE1022D"],
    "required": false,
    "fallback_manual_verified": true
  },
  "capabilities": {
    "channels": ["A", "B"],
    "recommended_channel": "B",
    "supports_rall": true,
    "supports_ascii_commands": true,
    "supports_time_constant_config": true,
    "supports_sensitivity_config": true
  },
  "safe_connect": {
    "commands": []
  },
  "safe_disconnect": {
    "commands": [],
    "close_transport": true
  }
}
```

说明：

```text
OE1022D 的连接由 Device Manager 创建；
RALL? 高频采集循环由 03_oe1022d_acquisition_prd 负责；
Device Registry 只提供连接、身份、状态、租约。
```

#### 6.3.3 Laser profile 示例

```json
{
  "schema_version": "0.2",
  "device_id": "laser.main",
  "kind": "laser",
  "display_name": "Main Laser Controller",
  "transport": {
    "type": "serial",
    "port": "COM4",
    "baud_rate": 9600,
    "data_bits": 8,
    "stop_bits": 1,
    "parity": "none",
    "timeout_ms": 1000,
    "line_ending": "\r\n"
  },
  "identity": {
    "query": null,
    "expected_contains": [],
    "required": false,
    "fallback_manual_verified": true
  },
  "capabilities": {
    "supports_enable": true,
    "supports_power_setpoint": true,
    "power_mw": { "min": 0.0, "max": 100.0 }
  },
  "safe_connect": {
    "commands": []
  },
  "safe_disconnect": {
    "commands": ["LASER:OFF"],
    "close_transport": true
  }
}
```

#### 6.3.4 Mag axis profile 示例

```json
{
  "schema_version": "0.2",
  "device_id": "mag.x",
  "kind": "mag_axis",
  "display_name": "Magnetic Axis X",
  "transport": {
    "type": "serial",
    "port": "COM5",
    "baud_rate": 115200,
    "data_bits": 8,
    "stop_bits": 1,
    "parity": "none",
    "timeout_ms": 1000,
    "line_ending": "\n"
  },
  "identity": {
    "query": "IDN?",
    "expected_contains": ["MAG", "X"],
    "required": false,
    "fallback_manual_verified": true
  },
  "capabilities": {
    "axis": "x",
    "current_a": { "min": -5.0, "max": 5.0 },
    "ramp_rate_a_per_s": { "min": 0.0, "max": 0.2 },
    "supports_output_enable": true,
    "supports_current_query": true
  },
  "safe_connect": {
    "commands": []
  },
  "safe_disconnect": {
    "commands": ["OUTP OFF"],
    "close_transport": true
  }
}
```

---

## 7. Outputs

Device Registry 输出以下内容。

### 7.1 Runtime connection status

运行时状态通过 API 返回，不直接写入 profile。

```json
{
  "device_id": "smb100a.main",
  "kind": "smb100a",
  "role": "rf_source",
  "status": "connected",
  "mode": "real",
  "transport": "tcp_scpi",
  "identity": {
    "matched": true,
    "raw_idn": "Rohde&Schwarz,SMB100A,...",
    "checked_at_wall": "2026-05-28T12:00:00+08:00"
  },
  "lease": {
    "state": "free",
    "owner": null
  },
  "last_error": null
}
```

---

### 7.2 station_snapshot.json

每次 run 开始前，必须保存设备快照：

```text
runs/<run_id>/metadata/station_snapshot.json
```

该文件用于回答：

```text
本次实验到底用了哪些设备？
每台设备当时的连接方式是什么？
IDN 是否匹配？
设备 capability 是什么？
设备是否处于 mock 模式？
```

示例结构：

```json
{
  "schema_version": "0.2",
  "run_id": "2026-05-28_001",
  "station_id": "nv_odmr_station_01",
  "created_at_wall": "2026-05-28T12:00:00+08:00",
  "devices": [
    {
      "device_id": "smb100a.main",
      "kind": "smb100a",
      "role": "rf_source",
      "mode": "real",
      "transport": {
        "type": "tcp_scpi",
        "host": "192.168.0.20",
        "port": 5025
      },
      "identity": {
        "matched": true,
        "raw_idn": "Rohde&Schwarz,SMB100A,..."
      },
      "capabilities_hash": "sha256:..."
    }
  ]
}
```

---

### 7.3 Connection events

所有连接相关动作必须写入 `events.jsonl`。

事件类型：

```text
device.connect.requested
device.connect.succeeded
device.connect.failed
device.disconnect.requested
device.disconnect.succeeded
device.disconnect.failed
device.identity.checked
device.identity.mismatch
device.lease.acquired
device.lease.released
device.lease.denied
device.status.changed
```

示例：

```json
{
  "event_type": "device.connect.succeeded",
  "time_wall": "2026-05-28T12:00:00+08:00",
  "time_monotonic_ns": 123456789000,
  "device_id": "smb100a.main",
  "mode": "real",
  "transport": "tcp_scpi",
  "message": "SMB100A connected and identity matched"
}
```

---

## 8. Runtime Model

### 8.1 架构位置

```text
Tauri / Web GUI
    |
    | invoke command
    v
Rust App Backend
    |
    v
Device Manager
    |-- Device Registry config loader
    |-- Runtime connection store
    |-- Resource lease manager
    |-- Transport factory
    |-- Device driver adapters
    |-- Mock device factory
```

Executor 和 Acquisition Core 不能绕过 Device Manager。

```text
Executor
    |
    | acquire lease
    v
Device Manager
    |
    v
SMB100A / Laser / Mag driver
```

```text
OE1022D Acquisition Core
    |
    | acquire acquisition lease
    v
Device Manager
    |
    v
OE1022D serial transport
```

---

### 8.2 Rust ownership model

推荐实现：

```text
DeviceManager
  owns RuntimeDeviceMap
  owns TransportHandle per connected device
  owns LeaseTable
  exposes async-safe API
```

每台设备运行时对象：

```rust
struct RuntimeDevice {
    device_id: DeviceId,
    kind: DeviceKind,
    mode: DeviceMode,
    profile: DeviceProfile,
    status: ConnectionStatus,
    transport: Option<TransportHandle>,
    identity: Option<IdentityState>,
    capabilities: DeviceCapabilities,
    lease: LeaseState,
    last_error: Option<DeviceError>,
}
```

原则：

```text
1. 底层 transport 不能复制到 GUI 或 Python。
2. 设备连接对象不能被多个 owner 同时持有。
3. 对同一设备的命令必须经过该设备的 command mutex / lease。
4. 需要高频采集的 OE1022D 可以申请长期 acquisition lease。
```

---

### 8.3 Connection lifecycle

状态机：

```text
unknown
  -> configured
  -> connecting
  -> connected_unverified
  -> connected_verified
  -> busy
  -> disconnecting
  -> disconnected
  -> error
```

简化状态用于 GUI：

```text
disabled
not_configured
disconnected
connecting
connected
busy
error
mock
```

连接流程：

```text
1. load station.json
2. load device_profile
3. validate profile schema
4. create transport according to mode
5. open transport
6. run identity query if available
7. match identity rule
8. execute safe_connect commands if configured
9. update status
10. emit events
```

断开流程：

```text
1. check lease state
2. reject disconnect if critical acquisition is running unless forced stop
3. execute safe_disconnect commands
4. flush transport if needed
5. close transport
6. update status
7. emit events
```

---

## 9. Data Model

### 9.1 Core identifiers

#### device_id

格式：

```text
<kind-or-group>.<name>
```

推荐：

```text
smb100a.main
oe1022d.main
laser.main
mag.x
mag.y
mag.z
```

约束：

```text
1. 全 station 唯一
2. 小写
3. 使用 dot 分层
4. 不使用空格
5. 不包含端口号
6. 不随 USB COM 编号变化而变化
```

---

### 9.2 Transport kinds

第一阶段支持：

```text
serial
  USB virtual COM / RS232 style devices

tcp_scpi
  SCPI over TCP socket, e.g. SMB100A LAN socket

visa
  reserved / optional / not first implementation path

mock
  fake device for test harness

replay
  historical state replay source
```

#### serial transport fields

```json
{
  "type": "serial",
  "port": "COM3",
  "baud_rate": 115200,
  "data_bits": 8,
  "stop_bits": 1,
  "parity": "none",
  "timeout_ms": 1000,
  "line_ending": "\r"
}
```

#### tcp_scpi transport fields

```json
{
  "type": "tcp_scpi",
  "host": "192.168.0.20",
  "port": 5025,
  "timeout_ms": 2000,
  "line_ending": "\n"
}
```

#### visa transport fields

```json
{
  "type": "visa",
  "visa_address": "TCPIP0::192.168.0.20::inst0::INSTR",
  "timeout_ms": 2000
}
```

v0.2 决策：

```text
visa transport 保留 schema 字段，但不是第一阶段主路径。
SMB100A 第一阶段优先 tcp_scpi。
```

---

### 9.3 Identity model

```json
{
  "query": "*IDN?",
  "expected_contains": ["SMB100A"],
  "expected_regex": null,
  "required": true,
  "fallback_manual_verified": false
}
```

规则：

```text
1. required = true 时，身份匹配失败必须拒绝进入 connected_verified。
2. required = false 时，身份失败可以进入 connected_unverified，但 GUI 必须显示警告。
3. fallback_manual_verified 只能用于确实没有 IDN 的设备。
4. manual_verified 设备必须在 station_snapshot.json 中标出。
```

---

### 9.4 Capability model

Capability 用于告诉上层：

```text
设备能做什么
参数范围是多少
某些高级功能是否可用
```

Capability 不是 recipe，也不是 safety limit 的替代品。

```json
{
  "supports_fm": true,
  "supports_frequency_sweep": true,
  "rf_frequency_hz": {
    "min": 100000.0,
    "max": 12750000000.0
  },
  "rf_power_dbm": {
    "min": -120.0,
    "max": 20.0
  }
}
```

安全规则：

```text
capability 描述设备理论能力；
safety limit 描述本实验站允许使用的范围；
recipe 必须同时满足 capability 和 safety limit；
当两者冲突时，取更严格者。
```

---

### 9.5 Lease model

资源租约用于防止同一设备被多个模块同时操作。

```json
{
  "lease_id": "lease_20260528_0001",
  "device_id": "oe1022d.main",
  "owner": "oe1022d_acquisition_core",
  "mode": "exclusive",
  "purpose": "rall_acquisition",
  "created_at_wall": "2026-05-28T12:00:00+08:00",
  "expires_at_wall": null
}
```

Lease mode：

```text
shared_read
  允许只读状态查询；不能发送改变设备状态的命令。

exclusive
  独占设备；用于 executor step、采集、危险命令。

acquisition
  长时间独占读循环；主要用于 OE1022D 高频采集。

maintenance
  手动维护 / 设备测试；阻止 executor 运行。
```

v0.2 简化策略：

```text
第一阶段可以只实现 exclusive + acquisition。
shared_read 可以暂缓，但 API 设计应预留。
```

---

## 10. API / Interface

### 10.1 GUI-facing API

GUI 只能通过 Tauri command 调用以下接口。

```text
list_devices()
get_device_status(device_id)
connect_device(device_id, mode)
disconnect_device(device_id, force=false)
reconnect_device(device_id)
query_identity(device_id)
validate_station_config()
scan_candidate_ports()
set_device_enabled(device_id, enabled)
```

返回结构必须可序列化为 JSON，不能暴露底层 serial handle / socket handle。

---

### 10.2 Executor-facing API

Executor 需要：

```text
require_devices(device_ids)
acquire_lease(device_id, owner, mode, purpose)
release_lease(lease_id)
send_command(lease_id, command)
query(lease_id, command)
get_capabilities(device_id)
get_station_snapshot()
```

Executor 规则：

```text
1. 没有 lease_id 不能发送设备命令。
2. lease owner 必须匹配调用者。
3. 设备 status 不是 connected_verified 时，默认拒绝危险命令。
4. mock 模式必须在 events 中明确记录。
```

---

### 10.3 Acquisition-facing API

OE1022D Acquisition Core 需要：

```text
acquire_acquisition_lease("oe1022d.main")
take_transport_for_acquisition(lease_id)
return_transport_after_acquisition(lease_id)
release_lease(lease_id)
```

重要边界：

```text
Device Manager 可以把 OE1022D transport 租给 Acquisition Core；
但 RALL? read loop、parser、ring buffer、raw bin writer 不在 Device Registry 内实现。
```

---

### 10.4 Test Harness API

测试需要：

```text
register_mock_device(device_id, mock_profile)
set_mock_response(device_id, command, response)
inject_disconnect(device_id)
inject_timeout(device_id)
inject_identity_mismatch(device_id)
read_connection_events()
```

---

## 11. Safety Rules

### 11.1 连接不等于输出

连接设备时默认不能打开危险输出。

特别是：

```text
SMB100A connect 不能打开 RF OUT。
SMB100A connect 不能打开 MOD ON。
Laser connect 不能打开激光输出。
Mag axis connect 不能突然改变电流。
OE1022D connect 不能自动改变灵敏度 / 时间常数，除非明确属于初始化 recipe。
```

---

### 11.2 safe_connect

`safe_connect` 只能执行低风险命令。

允许：

```text
query identity
query status
clear non-critical error queue
turn dangerous output off
set remote mode if required
```

禁止：

```text
turn RF on
turn laser on
increase laser power
increase magnetic current
start sweep
start acquisition loop
change OE1022D sensitivity without explicit executor step
```

---

### 11.3 safe_disconnect

断开前必须尽量让设备回到安全状态。

默认策略：

```text
SMB100A: OUTP OFF, MOD:STAT OFF
Laser: output off if supported
Mag axis: output off or ramp to configured safe current according to Safety PRD
OE1022D: stop acquisition lease, close serial transport
```

注意：

```text
磁场断开策略不能简单一律瞬间归零。
是否 ramp down、ramp rate、zero offset 由 Safety / Magnetic Planner PRD 定义。
Device Registry 只调用对应 safe_disconnect action。
```

---

### 11.4 Recipe 不能覆盖连接安全

Recipe 不能修改：

```text
device_id
transport
identity rule
safety limit
safe_disconnect
manual_verified flag
```

Recipe 只能引用设备：

```text
use_device: "smb100a.main"
```

不能直接写：

```text
port: COM3
host: 192.168.0.20
visa_address: ...
```

---

### 11.5 AI agent 限制

AI agent 可以生成：

```text
station.json 草案
profile.json 草案
mock profile
连接诊断说明
设备字段解释
```

AI agent 不能：

```text
直接连接硬件
直接扫描串口并打开设备
修改 safety_limits.json
把 unverified device 标记为 verified
把 recipe 中的设备连接参数硬编码为端口
绕过 Device Manager 生成硬件访问代码
```

---

## 12. Error Handling

### 12.1 错误分类

```text
config_error
  station.json / profile.json schema 错误

transport_error
  端口不存在、socket 打不开、权限不足

identity_error
  IDN 查询失败、IDN 不匹配

timeout_error
  命令超时、设备无响应

protocol_error
  响应格式不合法

lease_error
  设备被占用、lease owner 不匹配

safe_state_error
  safe_connect / safe_disconnect 命令失败

mock_error
  fake device 未配置响应或行为不一致
```

---

### 12.2 连接失败策略

当连接失败：

```text
1. status -> error
2. last_error 记录错误类型、message、时间
3. 写入 device.connect.failed event
4. GUI 显示错误和建议操作
5. 不允许 executor 使用该设备
6. required device 失败时，run 不能开始
```

---

### 12.3 身份不匹配策略

当 IDN 不匹配：

```text
required = true:
  拒绝连接为 verified
  status -> error or connected_unverified
  required run 不能开始

required = false:
  允许 connected_unverified
  GUI 显示黄色警告
  station_snapshot 必须标记 unverified
```

---

### 12.4 断开失败策略

当 safe_disconnect 命令失败：

```text
1. 记录 safe_state_error
2. 尽量继续关闭 transport
3. GUI 显示设备可能未进入安全状态
4. events.jsonl 记录失败命令
5. 对 SMB100A / Laser / Mag 这类危险设备，要求人工确认后再允许下一次 run
```

---

### 12.5 Lease 冲突策略

如果设备已有 lease：

```text
1. 新 lease 请求默认拒绝
2. GUI 可显示 owner / purpose
3. emergency stop 可以请求强制释放，但必须走 Safety Interlock
4. 强制释放必须写入 event
```

---

## 13. Test Harness

### 13.1 必须提供的 fake devices

第一阶段测试至少需要：

```text
fake SMB100A SCPI server
fake OE1022D serial device
fake laser serial device
fake mag axis X/Y/Z device
fake identity mismatch device
fake timeout device
fake disconnecting device
```

---

### 13.2 Unit tests

必须测试：

```text
station.json schema validation
profile.json schema validation
device_id uniqueness
transport config parsing
identity match success
identity mismatch rejection
capability parse
lease acquire / release
lease conflict rejection
safe_connect command sequence
safe_disconnect command sequence
```

---

### 13.3 Integration tests

必须测试：

```text
start app with all mock devices
connect all mock devices
disconnect all mock devices
simulate missing SMB100A
simulate missing OE1022D
simulate wrong IDN
simulate port busy
simulate TCP timeout
executor acquire SMB100A lease
OE1022D Acquisition Core acquire acquisition lease
GUI cannot send command without lease
station_snapshot generated before mock run
```

---

### 13.4 Hardware smoke tests

真实设备接入后，每台设备至少要有 smoke test：

```text
connect
query identity / status
safe_connect
get capabilities
safe_disconnect
reconnect
```

SMB100A smoke test 禁止打开 RF 输出。

OE1022D smoke test 禁止启动长时间 RALL? 采集，只做短命令测试；RALL? 压力测试归 03 PRD。

---

## 14. Acceptance Criteria

Device Registry & Connection v0.2 完成标准：

```text
1. 可以从 station.json 加载 SMB100A、OE1022D、Laser、Mag X/Y/Z 六类设备。
2. 每台设备有唯一 device_id。
3. 每台设备有 profile，包含 transport、identity、capabilities、safe_disconnect。
4. GUI 可以列出设备和连接状态。
5. 系统可以在无真实硬件时以 mock 模式启动。
6. fake SMB100A SCPI server 可以通过同一连接 API 连接。
7. fake OE1022D serial device 可以通过同一连接 API 连接。
8. IDN 匹配失败时，required device 不能进入 verified 状态。
9. Executor 没有 lease 时不能发送设备命令。
10. 同一设备不能被两个 exclusive lease 同时占用。
11. safe_disconnect 会被调用并记录 event。
12. run 开始前可以生成 station_snapshot.json。
13. required device disconnected 时，executor 拒绝开始 real run。
14. mock run 中所有设备状态必须在 events.jsonl 中标记为 mock。
15. 所有连接错误都能归类为 config / transport / identity / timeout / lease / safe_state 等类型。
```

---

## 15. Agent Constraints

### 15.1 Device Registry Agent

允许输入：

```text
02 Device Registry & Connection PRD
station.json schema
device profile examples
SMB100A command JSON
OE1022D command JSON
mock device requirements
```

允许输出：

```text
Rust data structs
schema files
mock device implementations
connection manager code
transport adapters
unit tests
integration tests
sample station.json
sample profiles
```

禁止：

```text
修改 GUI 页面结构
修改 recipe schema
实现 OE1022D 高频采集循环
实现 SMB100A ODMR sweep executor
修改 safety limit
让 AI 直接访问硬件
把真实端口硬编码进代码
绕过 lease 发送设备命令
```

---

### 15.2 Review checklist

每次修改本模块，review 必须检查：

```text
1. 是否引入了绕过 Device Manager 的硬件连接？
2. 是否把 runtime status 写回长期 profile？
3. 是否允许没有 lease 的命令发送？
4. 是否让 GUI 持有 serial/socket handle？
5. 是否把 recipe 和连接配置耦合？
6. 是否允许 AI 修改 safety limit？
7. 是否在 mock 模式和真实模式使用不同 API？
8. 是否记录 connection events？
9. 是否生成 station_snapshot？
10. 是否正确处理 required device 失败？
```

---

## 16. File Layout

建议文件布局：

```text
crates/
  device-registry/
    src/
      lib.rs
      manager.rs
      model.rs
      profile.rs
      station.rs
      identity.rs
      capability.rs
      lease.rs
      error.rs
      transport/
        mod.rs
        serial.rs
        tcp_scpi.rs
        visa.rs          # reserved
        mock.rs
      drivers/
        smb100a.rs
        oe1022d.rs
        laser.rs
        mag_axis.rs
      events.rs
    tests/
      station_validation_test.rs
      identity_test.rs
      lease_test.rs
      mock_connection_test.rs

config/
  station.json
  profiles/
    smb100a_main.profile.json
    oe1022d_main.profile.json
    laser_main.profile.json
    mag_x.profile.json
    mag_y.profile.json
    mag_z.profile.json

schemas/
  station.schema.json
  device_profile.schema.json
  device_status.schema.json
  device_event.schema.json
```

---

## 17. Open Questions

以下问题留到 v0.3 或实际硬件测试后决定：

```text
1. SMB100A 是否保留 VISA 作为用户可选 transport，还是只保留开发者 fallback？
2. OE1022D 的 IDN / handshake 是否足够稳定，是否需要 manual_verified 流程？
3. Laser controller 的真实协议和 safe output off 命令需要根据设备型号确认。
4. Mag X/Y/Z 是三台独立设备还是一个三轴控制器，需要现场确认。
5. Magnetic safe_disconnect 是 output off、ramp to zero，还是 hold current，需要由 Safety PRD 和实验风险决定。
6. shared_read lease 是否在 v0.2 实现，还是推迟到 v0.3。
7. scan_candidate_ports 是否只做诊断，还是允许自动绑定设备。
```

---

## 18. v0.2 Development Order

推荐实现顺序：

```text
1. 定义 Rust model：DeviceId / DeviceKind / TransportConfig / DeviceProfile / ConnectionStatus。
2. 实现 station.json 和 profile.json loader。
3. 实现 mock transport。
4. 实现 lease manager。
5. 实现 fake SMB100A SCPI server connection test。
6. 实现 fake OE1022D serial connection test。
7. 实现 GUI list/connect/disconnect API。
8. 实现 station_snapshot.json 输出。
9. 接入真实 SMB100A tcp_scpi smoke test。
10. 接入真实 OE1022D serial smoke test。
```

---

## 19. Summary

Device Registry & Connection 模块的核心边界是：

```text
它统一管理设备身份、连接、状态、capability、lease 和安全断开；
它不执行实验流程，不展开 recipe，不做高速采集，不画图，不运行 AI。
```

该模块完成后，系统应具备以下基础能力：

```text
1. 有设备时，可以安全、可审计地连接真实设备。
2. 没设备时，可以用 mock 设备完整跑开发和测试。
3. Executor 和 Acquisition Core 只能通过 lease 使用设备。
4. 每次实验都能保存 station_snapshot.json。
5. GUI、AI、Python 分析代码不能绕过 Device Manager 触碰硬件。
```

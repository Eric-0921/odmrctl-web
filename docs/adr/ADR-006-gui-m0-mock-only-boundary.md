# ADR-006: GUI-M0 采用 mock-only viewer 边界

## Status

Proposed

## Date

2026-05-29

## Decision Owner

ODMR Automation Project

## Related Documents

```text
docs/prd/00_main_prd_v0.2.md
docs/prd/08_gui_tauri_chart_prd_v0.3.md
docs/adr/ADR-001-tauri-ui.md
docs/adr/ADR-004-no-ai-live-hardware.md
docs/adr/ADR-005-raw-bin-before-csv.md
docs/gui/GUI-M0-spec.md
```

---

## 1. Context

本项目已经决定使用 Tauri + Web frontend + React + TypeScript + Rust backend boundary 构建桌面 GUI。ADR-001 已经明确 GUI 的定位是：

```text
控制面板 + 可视化界面 + 状态监控界面
```

而不是：

```text
实验执行器
硬件采集器
recipe compiler
安全决策中心
AI agent runtime
实时数据存储器
```

主 PRD 同时规定：

```text
GUI 不做实验逻辑
GUI 不直接访问仪器
GUI 不解析 OE1022D RALL? binary
GUI 不写 raw bin / index / events
AI 不直接控制硬件
executor 只接受通过 schema / safety / dry-run 的 resolved_recipe
```

当前仓库已有 mock run artifacts，可支撑 GUI-M0 展示：

```text
examples/recipes/basic_odmr_mock.recipe.json
examples/resolved/basic_odmr_mock.dry_run_plan.json
examples/safety/basic_odmr_mock.safety_report.json
examples/runs/basic_odmr_mock_executor_run/**
```

但是 `apps/desktop/` 仍处于初始阶段。此时如果过早接入真实设备、真实 executor 或真实 Tauri hardware command，会造成 GUI 边界失控。

---

## 2. Problem

GUI-M0 处于一个容易产生误区的阶段：

```text
Tauri 具备调用 Rust command 的能力
Rust 未来确实会拥有硬件能力
GUI 未来确实会有 Run / Stop / Device Connect 按钮
```

如果 M0 在页面骨架阶段就加入真实连接或真实控制行为，可能导致：

```text
1. GUI 被误当作实验执行器。
2. 前端按钮绕过 executor / safety interlock。
3. 设备访问路径散落到 Tauri commands 或 frontend utility 中。
4. 测试 harness 无法代表真实运行路径。
5. AI / agent 任务误修改硬件 crates 或引入 SCPI 行为。
6. M2 bring-up 前出现无法审计的危险控制入口。
```

因此需要一个单独 ADR 冻结 GUI-M0 的阶段性边界。

---

## 3. Decision

决定：**GUI-M0 必须实现为 mock-only viewer / UI skeleton。**

GUI-M0 只允许：

```text
展示静态 mock run artifacts
展示 recipe metadata
展示 dry-run plan
展示 safety report
展示 events.jsonl
展示 raw data artifact inventory
展示 fake device status cards
展示 disabled real-control buttons
展示 future backend integration placeholders
记录并公开 M0 boundaries
```

GUI-M0 禁止：

```text
真实设备连接
serial / USB / VISA / TCP socket 访问
SCPI 命令发送
SMB100A LAN 控制
OE1022D query / RALL? 读取
真实 executor 调用
真实 run / pause / stop
safety decision computation
safety limit override
raw bin / index / events 写入
CSV 写入
AI live hardware control
```

M0 中可以存在 Tauri shell，但任何 Tauri command 都只能返回：

```text
static mock data
application metadata
build/version metadata
```

M0 Tauri command 不得：

```text
导入硬件 crate
打开端口
创建 socket
调用 executor
写实验数据文件
读取任意用户指定 filesystem path
```

---

## 4. Scope of this decision

本 ADR 只约束 GUI-M0 阶段。

它不否定后续 M1/M2 中 GUI 通过安全后端调用真实能力，但规定后续真实能力必须经过：

```text
GUI user intent
→ Tauri command boundary
→ Rust backend API
→ device registry / resource lease
→ recipe executor
→ safety interlock
→ run event logger
→ raw-first data writer
```

GUI 不得直接持有硬件资源或绕过 executor。

---

## 5. Rationale

### 5.1 避免 GUI 重新成为实验逻辑中心

本项目重构的核心目标之一就是从旧式重 GUI 分离出来。如果 M0 直接实现硬件控制，后续很容易回到：

```text
按钮回调 = 实验逻辑
UI 状态 = 安全状态
前端行为 = 硬件行为
```

这与主 PRD 的实时链路最小化和 raw-first 数据原则冲突。

### 5.2 让 M0 聚焦信息架构与人机工效

M0 最有价值的交付物不是硬件控制，而是：

```text
页面结构是否正确
操作者能否看懂 mock run
safety 是否醒目
disabled state 是否明确
artifact mapping 是否可行
后端 API 插入点是否清晰
```

这些内容可以完全通过静态 mock data 验证。

### 5.3 防止 agent 误操作硬件边界

本项目会使用 AI / coding agent 辅助开发。若没有 M0 ADR，agent 可能为了让按钮“可用”而引入：

```text
serialport
socket
SCPI send helper
executor invoke helper
filesystem writer
```

这会直接破坏 ADR-004 的 no AI live hardware 边界。

### 5.4 保留后续真实后端接入路径

mock-only 不等于一次性原型。GUI-M0 的页面、组件、状态和数据 shape 应当为 M1/M2 预留后端接入位置。

M0 的静态数据结构应尽量模拟未来 backend DTO：

```text
RunSummary
DeviceStatusSnapshot
RecipeSummary
DryRunPlanSummary
SafetyReportSummary
RunEventRecord
ArtifactManifest
```

---

## 6. Consequences

### 6.1 Positive consequences

```text
降低误控硬件风险
降低 agent 越界实现风险
页面可以快速完成并审阅
mock / replay artifacts 可以先被产品化展示
M2 bring-up 前 GUI 不会污染硬件路径
后续真实 API 接入点更明确
```

### 6.2 Negative consequences

```text
M0 不能验证真实设备连接体验
M0 不能验证真实 executor latency
M0 不能验证 live chart streaming
M0 不能验证 emergency stop 的真实链路
部分控件只能以 disabled / placeholder 形式出现
```

这些代价是可接受的，因为 M0 目标是 GUI skeleton / mock viewer，而不是 M2 hardware bring-up。

---

## 7. Implementation rules

### 7.1 Allowed files for GUI-M0 implementation

Implementation agents may modify only:

```text
apps/desktop/**
package.json
pnpm-lock.yaml
package-lock.json
yarn.lock
docs/gui/**
README.md
```

README changes must only add GUI setup / local run instructions.

### 7.2 Forbidden files for GUI-M0 implementation

Implementation agents must not modify:

```text
crates/odmr-executor/**
crates/odmr-smb100a/**
crates/odmr-oe1022d/**
crates/odmr-device/**
crates/odmr-compiler/**
crates/odmr-safety/**
crates/odmr-logging/**
docs/prd/**
docs/adr/**
examples/**
```

### 7.3 Forbidden dependencies

GUI-M0 must not add dependencies whose purpose is real device access:

```text
serialport
visa / ni-visa wrappers
pyvisa / RsInstrument bindings
raw TCP socket SCPI helpers
USB / HID / GPIB device libraries
hardware polling utilities
```

### 7.4 Allowed mock data strategy

Preferred:

```text
apps/desktop/src/mock-data/**
```

Acceptable:

```text
bundled static frontend assets
hard-coded small TypeScript mock objects
```

Not allowed in M0:

```text
arbitrary filesystem browsing
user-selected run directory loading
backend file server for real experiment runs
```

---

## 8. UI boundary requirements

GUI-M0 must include a persistent text banner:

```text
GUI-M0 MOCK VIEWER — No hardware access. No executor connection. Real controls disabled.
```

Every real-control button must be disabled and labeled with reason:

```text
Start Run       disabled: Requires executor backend
Pause Run       disabled: Requires active executor run
Stop Run        disabled: Requires active executor run
Emergency Stop  disabled / visual-only: M0 has no hardware authority
Connect         disabled: M2 bring-up only
Probe           disabled: No serial / USB / VISA / socket probing in M0
RF Output ON    disabled: Forbidden in GUI-M0
MOD ON          disabled: Forbidden in GUI-M0
Auto Phase      disabled: Forbidden in GUI-M0
```

The About / Boundaries page must explicitly state:

```text
This GUI is mock-only.
It does not connect to devices.
It does not call executor.
It does not send SCPI.
It does not read OE1022D RALL?.
It does not write experiment data.
Future M2 integration must go through backend APIs, executor, and safety interlock.
```

---

## 9. Safety considerations

GUI-M0 displays safety state but does not compute safety.

Allowed:

```text
render decision from existing safety_report.json
render findings
render warning / reject / allow state
show operator-readable explanation
```

Forbidden:

```text
recompute safety decision
edit safety limits
override safety result
allow rejected plan to run
create new safety report as if authoritative
```

If mock safety report is missing or malformed, GUI-M0 should show:

```text
Safety decision: unknown
Reason: mock safety report unavailable or invalid
No run controls available in GUI-M0
```

---

## 10. Data considerations

M0 may show artifact inventory:

```text
manifest.json
events.jsonl
index.jsonl
raw/oe1022d.rawbin
metadata/**
```

M0 must not parse raw binary frames.

M0 must not write any experiment data files.

M0 may count JSONL entries if the bundled mock data is static and small. If count is not available, it may show `unknown` or a precomputed summary.

---

## 11. Future migration path

### 11.1 M1 read-only backend display

M1 may add backend APIs such as:

```text
get_mock_run_summary()
list_mock_artifacts()
get_mock_events()
get_replay_trace_preview()
```

These APIs remain read-only and must not call real devices.

### 11.2 M2 hardware bring-up

M2 may add real backend commands only after separate PRD/ADR/task approval:

```text
connect_device()
get_device_status_snapshot()
request_run_start()
request_run_stop()
request_safe_shutdown()
```

Even in M2, GUI commands must request intent; executor and safety own authority.

---

## 12. Acceptance criteria

This ADR is satisfied if GUI-M0 implementation has:

```text
1. mock-only banner on every page
2. disabled real-control buttons
3. no hardware dependencies
4. no executor dependency
5. no SCPI / serial / USB / VISA / socket implementation
6. no raw binary parser in frontend
7. no experiment data writer
8. static mock data source
9. About / Boundaries page documenting all restrictions
10. clear future integration placeholders
```

---

## 13. Rejection criteria

GUI-M0 implementation must be rejected if it contains:

```text
active device connection behavior
port scan behavior
socket connection to SMB100A
OE1022D command/query behavior
SCPI command sender
executor invocation
rawbin writer
CSV writer
AI live-control hook
frontend raw RALL? parser
mutation of safety limits
changes to hardware crates
```

---

## 14. Decision summary

GUI-M0 is deliberately a mock-only viewer.

This decision prevents premature hardware authority from leaking into the GUI while preserving a clear path for future backend integration.

The architectural boundary is:

```text
M0 GUI: display + interaction skeleton + mock artifact viewer
M1 GUI/backend: read-only backend display and replay integration
M2 backend: real hardware bring-up through executor + safety + device registry
```

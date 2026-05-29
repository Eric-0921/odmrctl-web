# Sub-PRD 08: GUI / Tauri / Chart PRD v0.3

> 文件定位：GUI 子 PRD  
> 项目代号：odmrctl-web / ODMR Automation  
> 当前版本：v0.3  
> 状态：Draft for review  
> 适用阶段：GUI-M0 mock-only viewer / M1 display integration / M2 hardware bring-up 前置约束  
> 关联文档：
>
> - `docs/prd/00_main_prd_v0.2.md`
> - `docs/adr/ADR-001-tauri-ui.md`
> - `docs/adr/ADR-004-no-ai-live-hardware.md`
> - `docs/adr/ADR-006-gui-m0-mock-only-boundary.md`
> - `docs/gui/GUI-M0-spec.md`

---

## 1. Purpose

本 PRD 定义 ODMR Automation 系统的桌面 GUI 需求，重点冻结 GUI-M0 的产品范围。

GUI-M0 的目标不是控制真实实验，而是构建一个可运行、可审查、可迭代的 **mock-only Tauri + React GUI skeleton / viewer**，用于展示已有 mock run artifacts、验证信息架构、验证操作员界面、验证安全状态表达，并为后续 M1/M2 接入真实后端提供稳定 UI 契约。

GUI 的长期定位是：

```text
控制面板 + 可视化界面 + 状态监控界面 + 数据运行浏览入口
```

GUI 不是：

```text
实验执行器
硬件驱动
采集线程
安全决策中心
recipe compiler
raw binary parser
AI live-control runtime
实时数据写入器
```

---

## 2. Background

主 PRD 已经明确系统要从旧式：

```text
重 GUI + 手动配置 + agent 直接操作式自动化
```

转向：

```text
轻 GUI + JSON recipe + Rust 高性能采集核心 + Python/AI 生成实验方案 + 可追溯数据资产
```

因此 GUI 必须避免重新膨胀成实验逻辑中心。

当前仓库已有 mock / dry-run / safety / run artifacts，可作为 GUI-M0 的静态数据源。GUI-M0 应先解决三个问题：

```text
1. 操作者能否看懂一次 ODMR mock run 的 recipe、dry-run、安全报告、事件与数据资产。
2. 前端能否清楚表达“当前不可控制硬件”的阶段性边界。
3. 页面、组件、状态、禁用控件与工业风格是否足以支撑后续真实后端接入。
```

---

## 3. Scope

### 3.1 GUI-M0 范围

GUI-M0 必须实现：

```text
1. Tauri + React + TypeScript 前端骨架
2. 工业蓝白灰风格的基础布局
3. 持久 mock-only 状态提示
4. 左侧导航 + 顶部状态栏 + 主内容区
5. Dashboard 页面
6. Devices 页面
7. Recipe 页面
8. Dry Run 页面
9. Safety 页面
10. Events 页面
11. Raw Data Preview 页面
12. About / Boundaries 页面
13. 静态 mock data 加载
14. 禁用态真实控制按钮
15. 本地运行说明
```

### 3.2 GUI-M0 数据来源

GUI-M0 应基于现有 example artifacts：

```text
examples/recipes/basic_odmr_mock.recipe.json
examples/resolved/basic_odmr_mock.dry_run_plan.json
examples/safety/basic_odmr_mock.safety_report.json
examples/runs/basic_odmr_mock_executor_run/manifest.json
examples/runs/basic_odmr_mock_executor_run/events.jsonl
examples/runs/basic_odmr_mock_executor_run/index.jsonl
examples/runs/basic_odmr_mock_executor_run/raw/oe1022d.rawbin
examples/runs/basic_odmr_mock_executor_run/metadata/**
```

M0 实现时优先把小型 JSON / JSONL snapshot 复制或转写到：

```text
apps/desktop/src/mock-data/**
```

M0 不要求动态读取任意 filesystem path。

---

## 4. Non-goals

GUI-M0 明确不做：

```text
真实设备连接
串口扫描
USB 扫描
VISA 连接
TCP socket SCPI 连接
SMB100A LAN 控制
OE1022D 命令读取
OE1022D RALL? binary 解析
真实 executor 调用
真实 run start / pause / stop
raw bin 写入
index.jsonl 写入
events.jsonl 写入
CSV 实时写入
recipe 编译
safety 判断
AI live hardware control
磁场 planner 实装
实时曲线流
文件选择器读取真实 run directory
```

M0 可以显示这些能力未来会存在，但所有真实控制入口必须处于禁用态，并标注原因。

---

## 5. User roles

### 5.1 Operator

操作者需要快速判断：

```text
当前是否真实连接硬件
当前 loaded recipe 是什么
mock dry-run 是否完整
safety 决策是什么
需要哪些设备
run 产生了哪些事件
run 产生了哪些数据 artifact
哪些按钮不可用，为什么不可用
```

### 5.2 Developer / Agent

开发者或 agent 需要通过 GUI-M0 验证：

```text
页面结构是否合理
mock artifacts 是否可以映射成 UI
状态模型是否清晰
M0 是否没有越界访问硬件
后续 backend API 应该接入哪里
```

### 5.3 Reviewer

审阅者需要检查：

```text
GUI 是否符合 PRD / ADR 边界
是否存在真实控制行为
安全状态是否足够醒目
禁用态是否明确
M1 / M2 接入点是否清楚
```

---

## 6. User workflows

### 6.1 Mock run review workflow

```text
Open GUI
→ See persistent GUI-M0 MOCK VIEWER banner
→ Dashboard shows current mock run summary
→ Open Recipe page
→ Review recipe metadata and resolved step count
→ Open Dry Run page
→ Review 201-step plan or paginated subset
→ Open Safety page
→ Review allow/reject decision and findings
→ Open Events page
→ Review events.jsonl records
→ Open Raw Data Preview page
→ Review rawbin / index / metadata inventory only
```

### 6.2 Device boundary review workflow

```text
Open Devices page
→ See fake SMB100A / OE1022D / Laser / Magnetic axes cards
→ Confirm connection status is mock / unavailable
→ Confirm Connect / Probe / Configure buttons are disabled
→ Confirm page states that M0 performs no serial / USB / VISA / socket probing
```

### 6.3 Control boundary review workflow

```text
Open Dashboard or Devices
→ See Start / Pause / Stop / Emergency Stop / Connect buttons
→ Confirm all real-control buttons are disabled
→ Confirm helper labels explain required future backend boundary
→ Confirm no real action occurs when clicking disabled controls
```

---

## 7. Global GUI requirements

### 7.1 Global layout

GUI-M0 must use a stable desktop application layout:

```text
Top status bar
Persistent mock-only banner
Left navigation rail
Main page content
Optional right detail panel only if simple
```

Minimum layout structure:

```text
┌────────────────────────────────────────────────────────────┐
│ Top Status Bar                                             │
├───────────────┬────────────────────────────────────────────┤
│ Left Nav      │ GUI-M0 MOCK VIEWER banner                  │
│               ├────────────────────────────────────────────┤
│ Dashboard     │ Page title + operator explanation          │
│ Devices       │ Cards / tables / JSON preview              │
│ Recipe        │ Disabled controls where relevant           │
│ Dry Run       │                                            │
│ Safety        │                                            │
│ Events        │                                            │
│ Raw Data      │                                            │
│ Boundaries    │                                            │
└───────────────┴────────────────────────────────────────────┘
```

### 7.2 Persistent top status fields

The top status bar must show:

```text
Project: ODMR Automation
Phase: M1 mock complete / M2 hardware bring-up pending
Mode: MOCK ONLY
Safety decision: allow / reject / unknown
Backend: static mock data
Run: basic_odmr_mock_executor_run
```

### 7.3 Persistent boundary banner

Every page must include a persistent banner:

```text
GUI-M0 MOCK VIEWER — No hardware access. No executor connection. Real controls disabled.
```

This banner must be visible without scrolling.

### 7.4 Navigation

Required navigation items:

```text
Dashboard
Devices
Recipe
Dry Run
Safety
Events
Raw Data Preview
About / Boundaries
```

Optional future items may appear as disabled nav entries:

```text
Live Chart       disabled: M1/M2 backend stream required
Run Control      disabled: executor backend required
Magnetic Planner disabled: future module
Settings         disabled: future module
```

---

## 8. Page requirements

### 8.1 Dashboard

Purpose:

```text
Give the operator a compact, high-confidence overview of the loaded mock run.
```

Required elements:

```text
Page title: Dashboard
Mode badge: MOCK ONLY
System phase card
Current mock run card
Safety decision card
Resolved step count card
Estimated duration card
Event count card
Artifact count card
Disabled run control strip
Recent event preview
Artifact inventory preview
```

Required disabled controls:

```text
Start Run       disabled: Requires executor backend
Pause Run       disabled: Requires active executor run
Stop Run        disabled: Requires active executor run
Emergency Stop  disabled / visual-only: M0 has no hardware authority
```

Dashboard should not include a live chart in M0. It may show a placeholder panel:

```text
Live trace preview unavailable in GUI-M0. Future M1/M2 will consume backend-downsampled trace stream.
```

---

### 8.2 Devices

Purpose:

```text
Show required devices and make hardware non-access explicit.
```

Required device cards:

```text
SMB100A microwave source
OE1022D lock-in amplifier
Laser controller placeholder
Magnetic axes placeholder
```

Each card must show:

```text
Device name
Device role
Required by current mock recipe: yes/no/unknown
Connection state: mock only / unavailable
Last known state: static snapshot / unavailable
Backend status: no hardware backend in M0
```

Required disabled controls:

```text
Connect           disabled: M2 bring-up only
Probe             disabled: no serial / USB / VISA / socket probing in M0
Configure         disabled: backend device registry required
Safe shutdown     disabled: no hardware authority in M0
RF Output ON      disabled: forbidden in GUI-M0
MOD ON            disabled: forbidden in GUI-M0
Auto Phase        disabled: forbidden in GUI-M0
```

Devices page must include a boundary notice:

```text
GUI-M0 contains no serial, USB, VISA, TCP socket, SCPI, or real device probing code.
```

---

### 8.3 Recipe

Purpose:

```text
Show the selected mock recipe and resolved recipe metadata without editing or compiling.
```

Required elements:

```text
Recipe name
Recipe file path or mock data source
Recipe hash if available
Resolved recipe hash if available
Station/profile if available
Required devices
Resolved step count = 201
Main sweep parameters if available
Read-only JSON preview
```

Forbidden in M0:

```text
recipe editing
recipe compilation
schema validation execution
AI recipe generation
safety limit modification
```

Permitted placeholder controls:

```text
Open Recipe       disabled: real file picker deferred
Compile Recipe    disabled: compiler backend required
Ask AI to Draft   disabled: AI workflow deferred and cannot bypass review
```

---

### 8.4 Dry Run

Purpose:

```text
Make the resolved execution plan inspectable before any real backend exists.
```

Required elements:

```text
Dry-run source path
Total step count
Estimated duration
Required devices
Safety-gated action summary if available
Step table
Optional JSON preview
```

Step table minimum columns:

```text
Index
step_id
device
action
parameters summary
duration / dwell if available
safety relevance
```

Acceptance requirement:

```text
The page must visibly render total step count = 201.
It may render a paginated subset, but total count must be obvious.
```

Forbidden in M0:

```text
executor invocation
real run start
device action dispatch
SCPI preview execution
```

---

### 8.5 Safety

Purpose:

```text
Make safety outcome prominent and reviewable.
```

Required elements:

```text
Safety decision badge: allow / reject / unknown
Safety report source
Finding count summary
Severity summary
Findings table
Operator-readable interpretation
```

Findings table minimum columns:

```text
severity
code
message
related step_id
related device
```

Safety page rules:

```text
Frontend displays existing safety report only.
Frontend does not compute safety decision.
Frontend does not modify safety limits.
Frontend must not allow user override in M0.
```

If decision is `allow`, still show that the system is mock-only. If decision is `reject`, dashboard top status and Safety page must both reflect reject state.

---

### 8.6 Events

Purpose:

```text
Display events.jsonl from the mock executor run.
```

Required elements:

```text
Event source path
Event count
Event table
Optional raw JSONL preview
```

Event table minimum columns:

```text
timestamp
event_type
step_id
level / severity
message
```

Rules:

```text
Handle missing optional fields gracefully.
Do not subscribe to live event stream in M0.
Do not write or append events.
```

---

### 8.7 Raw Data Preview

Purpose:

```text
Show data artifact inventory without parsing raw binary.
```

Required elements:

```text
run manifest summary
rawbin file name
rawbin size if available
index.jsonl entry count if available
metadata file list
summary file list
clear no-binary-parsing notice
```

Hard rule:

```text
GUI-M0 must not parse OE1022D RALL? raw binary frames.
```

Required notice:

```text
Raw binary parsing belongs to Rust backend / offline parser, not GUI-M0 frontend.
```

---

### 8.8 About / Boundaries

Purpose:

```text
Document M0 boundaries inside the running app.
```

Required content:

```text
This GUI is mock-only.
It does not connect to devices.
It does not call executor.
It does not send SCPI.
It does not read OE1022D RALL?.
It does not write experiment data.
Future M2 integration must go through backend APIs, executor, and safety interlock.
```

This page should list forbidden capabilities explicitly and link back to relevant local docs if implemented:

```text
docs/adr/ADR-001-tauri-ui.md
docs/adr/ADR-004-no-ai-live-hardware.md
docs/adr/ADR-006-gui-m0-mock-only-boundary.md
docs/gui/GUI-M0-spec.md
```

---

## 9. Visual and human-factors requirements

### 9.1 Visual direction

Use a restrained industrial software style:

```text
blue / white / gray palette
high information density without clutter
compact tables
clear status badges
strong disabled states
minimal animation
subtle borders
dense but readable typography
monospace for file paths, hashes, step IDs, event types
```

The style may be inspired by industrial software patterns, including Siemens-like blue-white engineering interfaces, but must not copy any proprietary assets, logos, icons, trademarks, or brand-specific visuals.

### 9.2 Accessibility

Minimum requirements:

```text
Do not convey status by color alone.
All status badges must include text.
Normal text contrast should target WCAG AA readability.
Keyboard focus must be visible.
Buttons must have clear accessible labels.
Disabled controls must have nearby explanatory text.
Tables must have clear column headers.
Long paths and hashes must wrap or truncate predictably.
```

---

## 10. Component requirements

Recommended component list:

```text
AppShell
TopStatusBar
SideNav
MockOnlyBanner
PageHeader
MetricCard
StatusBadge
DeviceCard
DisabledControlButton
BoundaryNotice
DataTable
JsonPreview
ArtifactList
EmptyState
ErrorState
LoadingState
```

These components are defined in detail in `docs/gui/GUI-M0-spec.md`.

---

## 11. Allowed implementation files

GUI-M0 implementation agents may modify only:

```text
apps/desktop/**
package.json
pnpm-lock.yaml
package-lock.json
yarn.lock
docs/gui/**
README.md
```

README changes must be limited to GUI setup and local run instructions.

---

## 12. Forbidden implementation files

GUI-M0 implementation agents must not modify:

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

Exception: this PRD and related docs may be edited by human design/review tasks, not by the GUI implementation task.

---

## 13. Acceptance criteria

GUI-M0 is complete when:

```text
1. GUI starts locally.
2. Dashboard renders mock run summary.
3. Persistent mock-only boundary banner is visible on all pages.
4. Devices page shows fake SMB100A and OE1022D cards.
5. Laser and magnetic axes placeholders are present.
6. All real-control buttons are disabled.
7. Recipe page shows basic_odmr_mock metadata if available.
8. Recipe page shows resolved step count = 201.
9. Dry Run page shows total step count = 201.
10. Dry Run page renders steps or a paginated subset.
11. Safety page renders decision allow/reject/unknown.
12. Safety findings render in table or clear list form.
13. Events page renders events.jsonl content.
14. Raw Data Preview shows artifact metadata only.
15. About / Boundaries page explicitly documents M0 constraints.
16. No hardware access code exists.
17. No serial / USB / VISA / TCP socket / SCPI code exists.
18. No real executor call exists.
19. No Rust hardware crates are imported into frontend or M0 backend.
20. README or docs/gui includes local run instructions.
```

---

## 14. Negative acceptance criteria

The task must be rejected if any of the following appear:

```text
active real device connect button
serialport dependency
VISA dependency
TCP socket SCPI implementation
SMB100A command sender
OE1022D query/read implementation
RALL? parser in frontend
executor invocation
real run start behavior
CSV/rawbin writer
hardware polling loop
AI live-control hook
modification to hardware crates
modification to PRD/ADR files by implementation agent
```

---

## 15. Future phases

### 15.1 M1: backend display integration

M1 may introduce read-only backend APIs for:

```text
run artifact listing
safe static file loading
mock replay stream
backend-downsampled chart data
status snapshot display
```

M1 still must not bypass executor or safety.

### 15.2 M2: hardware bring-up

M2 may introduce real device integration only through:

```text
Rust backend device registry
resource lease
safety interlock
executor authority
run event logger
raw-first data writer
explicit operator approval
```

GUI remains an intent and display surface, not the hardware controller.

---

## 16. Open questions

```text
1. Should GUI-M0 include a mock chart placeholder using synthetic downsampled data, or defer all charts to M1?
2. Should file browsing for prior run directories appear in M1 or M2?
3. Should About / Boundaries page be retained in production builds or hidden behind developer mode?
4. Should Siemens iX be used as a design reference only, or should a lightweight internal component style be built from scratch?
5. Should GUI-M0 snapshots be included in CI visual regression tests?
```

---

## 17. Summary

GUI-M0 is a mock-only viewer and UI skeleton. It should make the intended operator workflow visible without acquiring any hardware authority. The most important outcome is not visual polish alone, but a stable boundary:

```text
GUI displays operator intent and state.
Rust backend owns devices, executor, safety, acquisition, and data writing.
AI generates candidates only; it does not control live hardware.
```

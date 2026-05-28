# ADR-001: 使用 Tauri 构建桌面 GUI

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
docs/prd/07_data_logging_prd.md
docs/prd/08_gui_tauri_chart_prd.md
docs/prd/09_magnetic_planner_prd.md
docs/prd/10_safety_interlock_prd.md
docs/prd/11_harness_mock_replay_prd.md
docs/prd/12_agent_workflow_prd.md

docs/adr/ADR-002-rust-oe1022d-core.md
docs/adr/ADR-003-json-recipe-driven.md
docs/adr/ADR-004-no-ai-live-hardware.md
docs/adr/ADR-005-raw-bin-before-csv.md
```

---

## 1. Context

本项目是一个 ODMR 自动化实验系统。系统目标不是继续扩展旧式“重 GUI + 手动配置 + 脚本直接控制硬件”的结构，而是重构为：

```text
轻 GUI + JSON 实验 recipe + Rust 高性能采集核心 + Python/AI 生成和检查实验方案
```

第一阶段系统需要支持：

```text
设备连接
recipe 选择
schema 校验
dry-run 展示
run 控制
基础实时曲线
设备状态监控
magnetic planner
raw bin 数据落盘
实验事件日志
安全停止 / emergency stop
```

项目涉及多类设备和多条数据流：

```text
SMB100A 微波源
OE1022D 锁相放大器
Laser 控制器
Mag X/Y/Z 三轴磁场
recipe compiler
recipe executor
raw data logger
safety interlock
mock / replay test harness
```

GUI 的核心风险是再次膨胀为“实验逻辑中心”。如果 GUI 直接读设备、解析二进制数据、展开 recipe、执行安全判断，后续会产生几个问题：

```text
采集线程和 UI 线程耦合
实时数据和显示数据边界不清楚
AI / GUI / Executor 之间职责混乱
安全限制可能被 UI 状态绕过
测试 harness 难以覆盖真实运行路径
多 agent 开发时没有统一接口边界
```

因此，桌面 GUI 技术选型必须服务于以下架构边界：

```text
GUI 只负责交互和显示
Rust 后端负责设备边界、执行边界、采集边界和文件边界
Python/AI 只负责 recipe 生成、检查、解释和离线分析
硬件访问必须通过后端模块，不允许前端直接访问硬件
```

---

## 2. Decision

决定使用：

```text
Desktop Shell:       Tauri
Frontend:            Web frontend
Frontend Language:   TypeScript
Frontend Framework:  React，除非后续 ADR 单独变更
Chart Library:       uPlot 优先
Backend Boundary:    Tauri commands / events / channels
Backend Language:    Rust
```

GUI 的定位是：

```text
控制面板 + 可视化界面 + 状态监控界面
```

不是：

```text
实验执行器
硬件采集器
recipe compiler
安全决策中心
AI agent runtime
实时数据存储器
```

Tauri 应用结构采用：

```text
WebView frontend
    ↓ commands / events / channels
Rust Tauri backend
    ↓ internal service boundary
Device registry / Recipe executor / Acquisition core / Data logger / Safety interlock
```

---

## 3. Decision Details

### 3.1 前端允许做什么

前端允许负责：

```text
设备连接页
设备状态页
recipe 文件选择
recipe schema 校验结果展示
dry-run plan 展示
实验 run / pause / stop 控制
实时曲线显示
事件日志显示
run directory 浏览
magnetic planner 页面
安全状态和拒绝原因展示
实验完成后的导出入口
```

### 3.2 前端禁止做什么

前端禁止负责：

```text
直接访问 serial / USB / VISA / socket 硬件
直接发送 SCPI / 串口命令
解析 OE1022D RALL? binary frame
展开 recipe sweep
执行 recipe step
判断 safety limit 是否通过
写 raw bin
实时写 CSV
绕过 executor 直接改设备状态
在 live run 过程中调用 AI agent 控制硬件
```

### 3.3 Rust 后端必须负责什么

Rust 后端负责：

```text
设备连接抽象
设备状态快照
OE1022D 采集核心
raw frame parser
ring buffer
trace downsampling
recipe executor 调用边界
safety interlock 调用边界
run event 写入
raw bin / index / events 文件写入
前端 API 暴露
mock / replay harness 接入
```

### 3.4 Python / AI 负责什么

Python / AI 负责：

```text
生成 recipe 草案
解释 recipe
检查 recipe 合理性
生成分析 notebook
离线处理 parquet / csv
辅助生成文档和测试说明
```

Python / AI 不负责：

```text
live hardware control
OE1022D 高频采集
绕过 schema / safety / dry-run
修改 safety limit
直接连接真实设备执行实验
```

---

## 4. Rationale

### 4.1 Tauri 的进程边界符合本项目的架构边界

Tauri 使用 Web 前端和 Rust 后端构建桌面应用。官方架构说明中，Tauri 用 Rust 工具和 WebView 中渲染的 HTML 构建桌面应用，WebView 可通过消息传递与 Rust API 交互。

这与本项目的边界天然匹配：

```text
Web frontend 负责显示和用户意图
Rust backend 负责本地能力、设备能力、文件能力和安全边界
```

本项目需要避免“GUI 直接控制实验”。Tauri 的 command / event / channel 模型可以把 GUI 操作限制为后端 API 调用，而不是让前端拥有硬件能力。

### 4.2 比继续使用重 Python GUI 更适合长期维护

PyQt / PySide 适合快速原型，但本项目已经进入多设备、多线程、多 agent 协作阶段。如果继续把 GUI、硬件控制、采集、数据解析、文件写入、实验流程写在一个 Python GUI 进程中，后续容易出现：

```text
UI 卡顿影响采集
状态轮询打断采集
采集线程承担过多职责
实验逻辑散落在按钮回调里
mock / replay 不容易覆盖真实路径
```

Tauri 方案可以强制形成：

```text
前端按钮只是请求
Rust 后端才执行动作
Executor 才能改变实验状态
Safety interlock 才能决定是否允许危险命令
```

### 4.3 比纯 Rust GUI 更适合复杂配置界面

egui 这类纯 Rust GUI 可以减少技术栈数量，但本项目 GUI 不是简单控制面板，而是包含大量表单和可视化页面：

```text
设备连接表
recipe JSON 编辑 / 校验视图
dry-run timeline
多设备状态面板
magnetic planner
实时曲线
事件日志
run directory 浏览
```

Web 前端生态更适合实现这些复杂交互。Rust 保留在后端，负责性能、设备、采集和安全边界。

### 4.4 实时曲线只显示降采样数据，避免前端接触 raw frame

OE1022D 采集链路必须由 Rust 后端持有：

```text
OE1022D RALL? raw frame
    -> Rust acquisition thread
    -> parser thread
    -> ring buffer
    -> raw bin recorder
    -> downsampled trace API
    -> frontend chart
```

前端只接收类似下面的显示数据：

```json
{
  "trace": "oe1022d.ch_b.r",
  "window_s": 5.0,
  "max_points": 2000,
  "points": [[0.000, 0.0123], [0.010, 0.0125]]
}
```

前端永远不接收：

```text
RALL? raw binary frame
raw bin chunk
未解析串口字节流
```

这样可以避免：

```text
Web 前端解析二进制协议
UI 刷新影响采集稳定性
图表逻辑污染采集逻辑
```

### 4.5 Tauri 支持本地实验室软件的部署形态

本项目是本地实验室控制软件，不是云服务。Tauri 可以把 Web UI 和 Rust 后端打包成桌面应用，适合实验室电脑部署。

GUI 可以获得现代 Web UI 的开发效率，同时把本地文件、设备连接、进程管理、安全边界放在 Rust 后端处理。

---

## 5. Alternatives Considered

### 5.1 PyQt / PySide

#### 优点

```text
科学计算社区常见
原型开发快
Python 生态接入方便
已有项目迁移成本低
```

#### 缺点

```text
容易把 GUI、硬件控制、采集和实验逻辑写在一起
多线程采集和 UI 线程边界需要人工严格维护
长期维护时按钮回调容易变成实验执行逻辑
高频采集和实时绘图容易互相影响
前后端边界不如 Tauri + Rust 清晰
```

#### 结论

不作为新系统 GUI 主方案。可以保留旧 Python 工具用于离线分析、临时调试或设备命令验证。

---

### 5.2 Electron + Web Frontend

#### 优点

```text
Web UI 生态成熟
跨平台能力强
开发者熟悉度高
插件和调试生态丰富
```

#### 缺点

```text
运行时更重
Node/Electron 权限模型容易扩大前端能力边界
本项目后端核心本来就需要 Rust
不利于把硬件和安全能力集中收敛到 Rust 后端
```

#### 结论

不采用。Electron 的 Web UI 能力强，但本项目更需要轻量桌面壳和清晰 Rust 后端边界。

---

### 5.3 egui / iced / Slint 等纯 Rust GUI

#### 优点

```text
技术栈统一
Rust 后端集成直接
部署简单
性能可控
```

#### 缺点

```text
复杂表单和 JSON 编辑体验不如 Web 前端
图表、布局、状态面板生态不如 Web
magnetic planner 这类复杂页面开发成本可能更高
多 agent 协作时 UI 迭代速度可能变慢
```

#### 结论

不作为第一阶段主方案。后续如果需要极简仪器控制面板，可以单独评估纯 Rust GUI。

---

### 5.4 Web App + 本地服务

#### 优点

```text
前后端边界清楚
可用浏览器直接访问
部署灵活
```

#### 缺点

```text
实验室本地部署体验不如桌面应用
本地文件、设备权限和进程管理需要额外处理
用户可能误以为这是远程 / 云端系统
安全边界和网络暴露面更复杂
```

#### 结论

不作为第一阶段主方案。后续可以把部分只读监控页面扩展为局域网 Web dashboard，但 live control 仍应由本地桌面应用承担。

---

## 6. Consequences

### 6.1 正面影响

```text
GUI 和硬件边界更清楚
Rust 后端成为唯一硬件能力出口
前端不能直接绕过 safety interlock
实时采集链路可以独立于 UI 稳定运行
Web 前端适合复杂配置和图表界面
测试 harness 可以围绕后端 API 建立
多 agent 开发时模块边界更清楚
```

### 6.2 负面影响

```text
项目需要维护 TypeScript + Rust 两套主要代码
需要设计稳定的 Tauri command / event API
需要处理前后端状态同步
需要限制前端权限，避免 Tauri API 被滥用
开发者需要理解 WebView 与 Rust backend 的边界
```

### 6.3 接受的代价

接受以下代价：

```text
短期工程复杂度高于单体 Python GUI
需要额外定义前后端 API schema
需要对前端权限进行显式限制
需要为图表数据设计降采样接口
```

原因是这些代价换来的是长期可维护性、安全边界和采集稳定性。

---

## 7. Runtime Model

### 7.1 推荐进程 / 线程模型

```text
Tauri App Process
├── WebView Frontend
│   ├── device connection page
│   ├── recipe page
│   ├── dry-run page
│   ├── run dashboard
│   ├── real-time chart
│   └── magnetic planner page
│
└── Rust Backend
    ├── command API layer
    ├── app state store
    ├── device registry service
    ├── recipe executor client / service
    ├── safety interlock service
    ├── OE1022D acquisition thread
    ├── parser thread
    ├── ring buffer
    ├── data logger
    └── mock / replay harness adapter
```

### 7.2 前后端通信原则

```text
低频控制: Tauri command
状态变化: event
实时曲线: channel 或专门 trace subscribe API
大文件: 不通过 event 传输，由后端写文件并返回路径 / metadata
```

### 7.3 禁止的数据路径

```text
OE1022D raw frame -> Web frontend
serial bytes -> Web frontend
SCPI command string -> Web frontend direct send
raw bin -> Web frontend memory
AI response -> Executor direct run
```

---

## 8. API Boundary

### 8.1 前端调用后端的 API 示例

```text
connect_device(device_id)
disconnect_device(device_id)
get_station_snapshot()
validate_recipe(path)
compile_recipe(path)
get_dry_run_plan(resolved_recipe_id)
start_run(resolved_recipe_id)
pause_run(run_id)
stop_run(run_id)
emergency_stop()
subscribe_trace(trace_id, window_s, max_points)
open_run_directory(run_id)
export_run_csv(run_id)
```

### 8.2 后端推送给前端的事件示例

```text
device.status.changed
recipe.validation.finished
recipe.compile.finished
safety.rejected
run.started
run.step.started
run.step.finished
run.warning
run.error
run.finished
trace.updated
logger.file.rotated
```

### 8.3 API 设计要求

所有前后端 API 必须满足：

```text
参数结构化，不传自由拼接命令字符串
返回结构化错误码
危险动作必须可 dry-run 展示
危险动作必须经过 safety interlock
run 状态必须以后端为准
前端本地状态不能作为实验真实状态
```

---

## 9. Safety Rules

GUI 层必须遵守以下安全规则：

```text
GUI 不能修改 safety limit
GUI 不能覆盖 safety report
GUI 不能直接执行危险命令
GUI 不能在 safety failed 时启动 run
GUI 不能隐藏 dry-run 中的危险命令
GUI 必须始终显示 emergency stop 入口
GUI 必须显示当前 run safety status
GUI 必须显示 safety rejection reason
```

涉及以下内容时，必须通过后端安全层：

```text
SMB100A RF output ON
SMB100A power level change
Laser output ON / power change
Mag current change
Mag ramp command
OE1022D overload recovery
recipe executor start
manual override
```

---

## 10. Data Rules

GUI 不参与实时数据落盘。

实时数据路径固定为：

```text
acquisition thread -> raw bin / index / events -> post-run parquet / csv export
```

GUI 只读取：

```text
station_snapshot
run status
event tail
downsampled trace
export metadata
```

GUI 不写：

```text
raw bin
index.jsonl
events.jsonl
parsed.parquet
export.csv
```

CSV 只能作为实验后的导出格式，不能作为实时采集格式。

---

## 11. Implementation Guidelines

### 11.1 第一阶段页面

第一阶段 GUI 页面包括：

```text
Device Registry Page
Recipe Page
Dry-run Page
Run Dashboard
Live Trace Page
Event Log Page
Magnetic Planner Page
Run Files Page
Settings Page
```

### 11.2 图表策略

实时图表策略：

```text
uPlot 优先
刷新率 10–30 Hz
单图最多显示 1000–5000 点
Rust 后端负责降采样
前端不接触 raw frame
```

### 11.3 状态策略

状态源必须以后端为准：

```text
frontend local state: 只用于 UI 展示缓存
backend app state: 当前真实状态
events.jsonl: run 事实记录
station_snapshot.json: run 开始时设备状态快照
resolved_recipe.json: executor 执行依据
```

---

## 12. Test Requirements

### 12.1 GUI 测试

必须覆盖：

```text
设备连接状态显示
recipe schema 校验结果显示
dry-run 展示
safety rejection 展示
run start / stop 按钮状态
emergency stop 可见性
trace subscribe / unsubscribe
事件日志滚动
run files 展示
```

### 12.2 Backend API 测试

必须覆盖：

```text
mock device connection
fake OE1022D RALL frame
fake SMB100A SCPI server
fake laser serial
fake mag axis
recipe validation tests
dry-run tests
safety rejection tests
raw bin replay tests
timestamp alignment tests
```

### 12.3 禁止路径测试

必须有测试或代码审查规则确认：

```text
前端不能 import 硬件通信模块
前端不能构造任意 SCPI 直发接口
前端不能解析 raw frame
前端不能写 raw data 文件
AI 输出不能直接进入 executor
未通过 safety_report 的 recipe 不能启动 run
```

---

## 13. Acceptance Criteria

本 ADR 对应的验收标准：

```text
项目能启动 Tauri 桌面应用
前端能通过 Tauri command 获取 station snapshot
前端能展示 mock device status
前端能加载 recipe 并显示校验结果
前端能显示 dry-run plan
前端能启动 mock run
前端能接收 run events
前端能显示 downsampled live trace
前端不能直接访问硬件接口
前端不能解析 raw RALL? frame
emergency stop 在 run 页面始终可见
safety rejection 能被清楚展示
```

---

## 14. Agent Constraints

AI agent 可以：

```text
生成 GUI 页面草案
生成 TypeScript 类型定义
生成 Tauri command binding 草案
生成 mock 数据
生成 recipe 示例
生成 dry-run 展示文案
解释 safety rejection
生成测试用例草案
```

AI agent 不可以：

```text
直接生成绕过后端的硬件访问代码
把 SCPI / serial 任意命令发送能力暴露给前端
修改 safety limit
让 GUI 直接执行 recipe step
让 GUI 直接解析 raw frame
把 CSV 作为实时采集格式
让 AI 输出直接进入 live executor
```

---

## 15. Review Triggers

出现以下情况时，需要重新评估本 ADR：

```text
Tauri 无法满足实时曲线性能需求
Tauri 的权限模型无法满足安全边界要求
WebView 在目标实验室电脑上兼容性不足
前后端通信成为采集瓶颈
项目决定转为浏览器访问的局域网系统
项目决定完全放弃桌面应用形态
```

---

## 16. References

```text
Tauri Architecture
https://v2.tauri.app/concept/architecture/

Tauri Security
https://v2.tauri.app/security/

Tauri: Calling Rust from the Frontend
https://v2.tauri.app/develop/calling-rust/

Tauri: What is Tauri?
https://v2.tauri.app/start/
```

---

## 17. Final Decision Summary

本项目第一阶段桌面 GUI 采用：

```text
Tauri + TypeScript Web frontend + Rust backend
```

核心原因：

```text
Tauri 能把 GUI 限制为显示和用户意图层
Rust 后端能集中持有硬件、采集、文件和安全边界
Web 前端适合复杂配置、JSON 编辑和实时曲线展示
该方案最符合“轻 GUI + Rust 核心 + JSON recipe + AI 不直接控制硬件”的总体架构
```

# GUI-M0 Pages Documentation

> 本文档汇总 **odmrctl-web** 前端桌面应用（GUI-M0 Mock Viewer）所有路由页面的视觉截图与代码构建说明。
>
> 截图时间：2026-06-05  
> 视口尺寸：1920 × 1200（外置 4K 显示器环境）  
> 构建产物：`apps/desktop/dist/`（Vite 静态构建）  
> 截图路径：`screenshots/*.png`

---

## 全局布局架构

所有页面共享统一的 **AppShell** 布局（`src/components/AppShell.tsx`）：

```
┌────────────────────────────────────────┐
│ TopStatusBar  (phase / mode / backend) │
├────────┬───────────────────────────────┤
│        │ MockOnlyBanner                │
│ SideNav├───────────────────────────────┤
│        │                               │
│ (11    │      <Route Page Content>     │
│  active│                               │
│  items)│                               │
│        │                               │
│ + 4    │                               │
│ disabled                              │
│ items) │                               │
└────────┴───────────────────────────────┘
```

- **TopStatusBar**: 显示 `Phase: M1 mock complete / M2 hardware bring-up pending`、`GUI-M0 MOCK`、`Allow`、`Backend: bundled static mock data`、`Run: basic_odmr_mock_executor_run`
- **MockOnlyBanner**: 蓝色提示条 `M4.1 DRY-RUN VIEWER — No hardware access. No executor connection. Real controls disabled.`（所有页面顶部固定）
- **SideNav**: 232px 固定宽度，11 个可点击路由 + 4 个禁用占位项（Live Chart、Run Control、Magnetic Planner、Settings）

---

## 路由清单

| # | 页面 | 路由 | 截图文件 | 数据来源 | Tauri 命令 |
|---|------|------|----------|----------|-----------|
| 1 | [Dashboard](#dashboard) | `/` | `dashboard.png` | `getRunSummary()` mock helper | ❌ 无 |
| 2 | [Devices](#devices) | `/devices` | `devices.png` | 硬编码 `devices[]` 数组 | ❌ 无 |
| 3 | [Recipe](#recipe) | `/recipe` | `recipe.png` | `getRecipe()` + `getRunSummary()` | ❌ 无 |
| 4 | [Dry Run](#dry-run) | `/dry-run` | `dry-run.png` | `getDryRunPlan()` + `getDryRunSteps()` | ❌ 无 |
| 5 | [Safety](#safety) | `/safety` | `safety.png` | `getSafetyReport()` | ❌ 无 |
| 6 | [Events](#events) | `/events` | `events.png` | `getEvents()` + `getRunSummary()` | ❌ 无 |
| 7 | [Raw Data Preview](#raw-data-preview) | `/raw-data` | `raw-data.png` | `getRunManifest()` + `getRawArtifactSummary()` + `getIndexEntries()` | ❌ 无 |
| 8 | [Analysis Viewer](#analysis-viewer) | `/analysis-viewer` | `analysis-viewer.png` | 用户选择的 M3.6 分析目录（JSON 文件） | ✅ `pick_analysis_directory` + `read_analysis_directory` |
| 9 | [Recipe Viewer](#recipe-viewer) | `/recipe-viewer` | `recipe-viewer.png` | 默认示例 JSON / 用户选择的 recipe 文件 | ✅ `pick_recipe_file` + `read_recipe_file` |
| 10 | [System Scan](#system-scan) | `/system-scan` | `system-scan-*.png` (7 tabs) | `src/mock-data/m5b/*.json` bundle | ❌ 无 |
| 11 | [About / Boundaries](#about--boundaries) | `/about` | `about.png` | 100% 硬编码 JSX | ❌ 无 |

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 构建工具 | Vite 5 + TypeScript 5 |
| 框架 | React 18 |
| 路由 | react-router-dom v6 |
| 样式 | CSS-in-JS（inline style，CSS 变量在 `styles/tokens.css`） |
| 图表 | `recharts`（仅 Analysis Viewer） |
| 桌面壳 | Tauri v2（Rust 后端） |
| 包管理 | pnpm |

---

## 截图生成方式

```bash
cd apps/desktop
pnpm build                    # Vite 构建到 dist/
cd dist && python3 -m http.server 8765  # 静态服务器
python3 screenshot_all_pages.py          # Playwright 自动截图
```

截图脚本：`screenshot_all_pages.py`
- 浏览器：Playwright Chromium
- 视口：1920 × 1200
- 路由截图：`full_page=False`（固定视口）
- System Scan tabs：通过 `page.locator("button", has_text=...)` 自动点击切换

---

## 数据分层

```
Mock data (bundled at build time)
  ├── src/mock-data/helpers.ts        # getRunSummary / getRecipe / getDryRunPlan 等
  ├── src/mock-data/m5b/              # M5B-B JSON artifacts (6 files)
  └── src/routes/*Page.tsx            # 直接 import mock helper 或 JSON

Tauri file loading (runtime user selection)
  ├── AnalysisViewerPage.tsx          # pick_analysis_directory + read_analysis_directory
  └── RecipeViewerPage.tsx            # pick_recipe_file + read_recipe_file
```

---

## Hard Boundaries (GUI-M0)

所有页面统一遵守以下架构约束：

| 约束 | 说明 |
|------|------|
| 无硬件访问 | 无 serial / USB / VISA / TCP socket 代码 |
| 无 SCPI 发送 | 前端不构造 SCPI 字符串 |
| 无 executor 调用 | 不触发实验执行 |
| 无安全覆盖 | 不修改安全决策 |
| 无数据写入 | 不写文件、不创建目录 |
| 只读为主 | 9/11 页面为纯 mock 数据展示 |
| 文件加载只读 | Analysis / Recipe Viewer 仅读用户选中的文件，不修改 |

---
---

---

# Dashboard

## Route
`/` — 应用默认首页

## Screenshot
![Dashboard](../../screenshots/dashboard.png)

## Visual Description

页面展示 **GUI-M0 模拟运行概览**，采用 3 列网格布局的统计卡片：

| 卡片 | 值 |
|------|-----|
| Run name | `basic_odmr_mock_executor_run` |
| System phase | `M1 mock complete / M2 pending` |
| Current mode | `GUI-M0 MOCK ONLY` |
| Safety decision | `allow` |
| Resolved steps | `201` |
| Estimated duration | `201 s` |
| Event count | `407` |
| Artifact count | `8` |
| Required devices | `smb100a_01` |

底部有 4 个禁用按钮（Start Run / Pause Run / Stop Run / Emergency Stop），每个按钮下方标注禁用原因（如 "requires executor backend"、"no hardware authority in GUI-M0"）。

页面顶部有全局状态栏显示：`Phase: M1 mock complete / M2 hardware bring-up pending`、`GUI-M0 MOCK`、`Allow`、`Backend: bundled static mock data`、`Run: basic_odmr_mock_executor_run`。

左侧导航栏高亮 "Dashboard"。

## Code Structure

- **文件**: `src/routes/DashboardPage.tsx` (95 行)
- **数据**: `getRunSummary()` from `src/mock-data/helpers` — 纯静态 mock 数据
- **组件**: 无外部 UI 库依赖，纯 CSS-in-JS 内联样式
- **交互**: 无 Tauri 命令调用；4 个操作按钮均为 `disabled`，cursor 设为 `not-allowed`
- **布局**: CSS Grid `repeat(3, 1fr)` 卡片网格 + Flexbox 按钮行

---

# Devices

## Route
`/devices`

## Screenshot
![Devices](../../screenshots/devices.png)

## Visual Description

页面展示 **6 台实验设备的静态卡片**，采用 2 列网格布局：

1. **SMB100A** — RF / microwave signal generator
2. **OE1022D** — DSP lock-in amplifier / acquisition source
3. **Laser Controller** — laser controller placeholder（optional）
4. **Mag X (MAYNUO M8812)** — X-axis current source
5. **Mag Y (MAYNUO M8812)** — Y-axis current source
6. **Mag Z (MAYNUO M8812)** — Z-axis current source

每张卡片显示：设备名称、Role、Required by recipe（yes/no）、Connection status（均显示 "unavailable in GUI-M0"）、Mock status（static snapshot only）、Last known state。

底部有每个设备的禁用操作按钮组：
- SMB100A: Connect / Probe / Configure / Output ON / MOD ON
- OE1022D: Connect / Probe / Configure
- Laser: Connect / Emission ON
- Mag X/Y/Z: Connect / Set current / Output ON

所有按钮均为 `disabled`，原因标签包括 "M2 bring-up only"、"Forbidden in GUI-M0"、"Mock viewer only"。

页面顶部有一条蓝色 info banner：`No serial / USB / VISA / TCP socket probing exists in GUI-M0`。

## Code Structure

- **文件**: `src/routes/DevicesPage.tsx` (154 行)
- **数据**: 两个硬编码数组 — `devices[]`（6 台设备元数据）和 `disabledControls`（每设备禁用按钮映射）
- **组件**: 纯 CSS-in-JS，无外部 UI 库
- **交互**: 无 Tauri 命令；所有按钮 `disabled` + `cursor: not-allowed`
- **布局**: CSS Grid `repeat(2, 1fr)` 卡片网格，卡片内 Flexbox 垂直排列按钮组

---

# Recipe

## Route
`/recipe`

## Screenshot
![Recipe](../../screenshots/recipe.png)

## Visual Description

页面展示 **Recipe 元数据与扫频参数**，包含四个区域：

1. **Summary cards** (4 列): Recipe name、Schema version、Resolved steps、Required devices
2. **Main sweep parameters** (4 列网格): Axis、Start (GHz)、Stop (GHz)、Step (MHz)、Order、Points
3. **Recipe metadata table**: Recipe ID、Recipe hash（截断显示）、Schema version、Station / profile、Experiment type、Description
4. **Recipe JSON preview** (可折叠): 点击 "Show JSON / Hide JSON" 展开/收起完整 JSON，带 `maxHeight: 400` 滚动区域

底部有 3 个禁用按钮：Open Recipe、Compile Recipe、Ask AI to Draft，分别标注禁用原因。

## Code Structure

- **文件**: `src/routes/RecipePage.tsx` (214 行)
- **数据**: `getRecipe()` + `getRunSummary()` from `src/mock-data/helpers`
- **状态**: `useState(false)` 控制 JSON preview 折叠
- **组件**: 无外部 UI 库；纯 table + pre + 卡片布局
- **交互**: 无 Tauri 命令；"Show JSON" 按钮是唯一可用交互（本地状态切换）
- **布局**: CSS Grid `repeat(4, 1fr)` + 可折叠 overflow hidden 容器

---

# Dry Run

## Route
`/dry-run`

## Screenshot
![Dry Run](../../screenshots/dry-run.png)

## Visual Description

页面展示 **Dry-run 执行计划概览与步骤明细**：

1. **Summary cards** (4 列):
   - Total steps
   - Estimated duration (s)
   - Required devices
   - Gated actions

2. **Steps table**: 7 列表头 — `#`、`step_id`、`device`、`action`、`parameters`、`duration`、`safety`
   - 每行解析 `device: action` 格式显示 device 和 actionName
   - sweep_coordinate 以 `key=value` 形式显示
   - safety 列统一显示绿色 `safe` badge
   - 最多显示 50 行，底部提示 "Showing 50 of N steps"

## Code Structure

- **文件**: `src/routes/DryRunPage.tsx` (134 行)
- **数据**: `getDryRunPlan()` + `getDryRunSteps()` from `src/mock-data/helpers`
- **工具函数**: `parseDeviceAction()` 解析 "device: action" 字符串
- **组件**: 无外部 UI 库
- **交互**: 无 Tauri 命令；纯只读表格
- **布局**: CSS Grid summary cards + 全宽 table

---

# Safety

## Route
`/safety`

## Screenshot
![Safety](../../screenshots/safety.png)

## Visual Description

页面展示 **安全审查决策与发现项**：

1. **Decision banner**: 顶部大横幅显示 `Safety decision: Allow`（绿色）或 `Reject`（红色），附带说明 "Displayed from existing mock safety report. Frontend does not compute safety."

2. **Summary cards** (4 列):
   - Checked steps
   - Checked actions
   - Info / Warning / Error 计数
   - Source 文件名

3. **Findings table**: 5 列表头 — `severity`、`code`、`message`、`step_id`、`device`
   - severity 列带颜色 badge（info=绿、warning=黄、error=红）
   - code / step_id 使用等宽字体
   - 若 findings 为空则显示提示文本

## Code Structure

- **文件**: `src/routes/SafetyPage.tsx` (140 行)
- **数据**: `getSafetyReport()` from `src/mock-data/helpers`
- **工具函数**: `severityStyle()` 根据 severity 返回背景色和文字色 CSS 变量
- **组件**: 无外部 UI 库
- **交互**: 无 Tauri 命令；纯只读展示
- **布局**: 彩色 decision banner + 4 列 grid cards + 全宽 findings table

---

# Events

## Route
`/events`

## Screenshot
![Events](../../screenshots/events.png)

## Visual Description

页面展示 **实验事件日志**：

1. **Summary cards** (3 列 flex):
   - Event count
   - Source (`events.jsonl`)
   - Run ID

2. **Events table**: 5 列表头 — `timestamp` (ISO 8601)、`event_type`、`step_id`、`level`、`message`
   - timestamp 使用等宽字体小字号
   - level 列带颜色 badge（info=绿、warning=黄、error/danger=红）
   - event_type / step_id 使用等宽字体
   - 最多显示 10 条，底部提示 "Showing 10 of N events"

## Code Structure

- **文件**: `src/routes/EventsPage.tsx` (137 行)
- **数据**: `getEvents()` + `getRunSummary()` from `src/mock-data/helpers`
- **工具函数**: `levelStyle()` 根据 level 返回 badge 样式；timestamp 通过 `new Date(ms).toISOString()` 转换
- **组件**: 无外部 UI 库
- **交互**: 无 Tauri 命令；纯只读展示
- **布局**: Flexbox summary cards + 全宽 table

---

# Raw Data Preview

## Route
`/raw-data`

## Screenshot
![Raw Data Preview](../../screenshots/raw-data.png)

## Visual Description

页面展示 **实验产物目录清单与元数据**：

1. **Warning banner**: 黄色边框提示条，显示 raw data 预览说明

2. **Summary cards** (4 列):
   - Rawbin file 名
   - Rawbin size (bytes)
   - Index entries 数量
   - Metadata files 数量

3. **Artifact inventory table**: 5 列 — `path`、`type`、`size`、`role`、`parsed by GUI-M0`
   - 8 种产物类型: manifest、events、index、rawbin、metadata×5
   - "parsed" 列显示 Yes（绿色）或 No（红色）badge
   - rawbin 显示实际字节数，其余显示 "—"

4. **Manifest metadata table**: key-value 对 — Run ID、Recipe hash（截断）、Resolved recipe ID、Safety report ID、Created at (ISO 时间)

## Code Structure

- **文件**: `src/routes/RawDataPreviewPage.tsx` (172 行)
- **数据**: `getRunManifest()` + `getRawArtifactSummary()` + `getIndexEntries()` from `src/mock-data/helpers`
- **组件**: 无外部 UI 库
- **交互**: 无 Tauri 命令；纯只读展示
- **布局**: Warning banner + 4 列 grid cards + 两个全宽 table

---

# Analysis Viewer

## Route
`/analysis-viewer`

## Screenshot
![Analysis Viewer](../../screenshots/analysis-viewer.png)

## Visual Description

页面初始状态显示 **"Select an M3.6 analysis directory to view results."** 提示和一个蓝色按钮 **"Select Analysis Directory"**。

点击按钮后：
1. 调用 Tauri 原生文件选择器 (`pick_analysis_directory`) 打开文件夹选择对话框
2. 选中目录后通过 `read_analysis_directory` 命令读取分析数据
3. 加载完成后展示完整分析结果，包括：
   - **Quality grade banner**: 显示通过/失败状态和关键指标（parse failure rate、all runs passed 等）
   - **Quality flags grid**: 8 个布尔指标卡片（missing artifact、failed run、parse failures、audit mismatch、unsafe final state、csv present、magnetic command present、frequency grid mismatch）
   - **Frequency-vs-signal chart**: 使用 `recharts` 绘制的折线图，展示各频率点的 B_x / B_y 信号
   - **Run overlay summary table**: 各频率组的统计摘要（mean、std、min、max）
   - **Source runs table**: 参与分析的各 run 的来源信息

底部有文件路径显示和重新加载按钮。

## Code Structure

- **文件**: `src/routes/AnalysisViewerPage.tsx` (369 行)
- **Tauri 命令**:
  - `invoke("pick_analysis_directory")` — 原生文件夹选择器
  - `invoke("read_analysis_directory", { path })` — Rust 后端读取分析目录
- **Rust 后端** (`src-tauri/src/main.rs`): `read_analysis_directory` 解析 5 种分析产物文件
  - `quality_flags.json`（必需）
  - `odmr_like_analysis_summary.json`（必需）
  - `run_overlay_summary.json`（必需）
  - `spectrum_points.jsonl`（必需，JSONL 格式逐行解析）
  - `export_manifest.json`（可选）
- **类型**: `AnalysisData` from `src/types/analysis.ts` — TypeScript/Rust 类型必须保持同步
- **外部库**: `recharts` (LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer)
- **工具函数**: `formatVoltage()` / `pickVoltageUnit()` — 自动选择 mV/V 单位
- **状态**: 标签联合类型 `LoadState` = empty | loading | error | loaded
- **布局**: 居中初始状态 → 加载完成后多 section 纵向排列

---

# Recipe Viewer

## Route
`/recipe-viewer`

## Screenshot
![Recipe Viewer](../../screenshots/recipe-viewer.png)

## Visual Description

页面初始状态展示 **默认示例 recipe JSON** 在文本编辑区，包含完整的验证和预览面板：

1. **顶部操作栏**: "Open Recipe File"（调用 Tauri 文件选择器）和 "Validate" 按钮
2. **JSON 编辑区**: `textarea` 预填充 `EXAMPLE_RECIPE_JSON`（two_device_odmr_like_sweep_recipe 示例），支持用户粘贴/编辑
3. **Validation badges**: 4 个状态 badge — JSON parse、Shape check、Value check、Overall（通过/失败）
4. **Errors / Warnings 列表**: 验证失败时显示具体错误信息
5. **Recipe summary cards**: Recipe ID、Schema version、Devices count、Steps estimate
6. **Preview panels** (4 个标签页或垂直排列):
   - **Resolved Preview**: 展开后的步骤预览
   - **Dry-Run Preview**: 预计执行计划
   - **Safety Preview**: 安全检查结果预览
   - **Command Plan Preview**: 设备命令序列预览

## Code Structure

- **文件**: `src/routes/RecipeViewerPage.tsx` (784 行)
- **Tauri 命令**:
  - `invoke("pick_recipe_file")` — 原生文件选择器（过滤 .json）
  - `invoke("read_recipe_file", { path })` — 读取 recipe 文件为纯文本
- **Rust 后端** (`src-tauri/src/main.rs`): `read_recipe_file` 仅读取文件内容为字符串，不做解析
- **本地工具函数**:
  - `validateRecipe()` — 客户端 JSON parse + shape + value 验证
  - `buildResolvedPreview()` / `buildDryRunPreview()` / `buildSafetyPreview()` / `buildCommandPlanPreview()` — 客户端模拟编译/安全/干跑预览
- **类型**: `RecipeValidationResult`、`ResolvedPreview`、`DryRunPreview`、`SafetyPreview`、`CommandPlanPreview` from `src/types/recipe.ts`
- **状态**: `useState` 管理 JSON 文本内容、验证结果、各预览面板状态
- **布局**: 垂直分区 — 操作栏 → textarea → 验证状态 → 错误列表 → Summary cards → 4 个预览面板

---

# System Scan

## Route
`/system-scan`

## Screenshots

System Scan 页面采用 **7-Tab 标签页** 布局，所有数据来自 M5B-B mock artifact bundle。

### Overview
![Overview](../../screenshots/system-scan-overview.png)

顶部绿色 `Safety decision: ALLOW` 横幅，下方 8 张 summary cards（3-3-2 布局）：
- Recipe ID、Total steps (15)、Sweep points (9)、Estimated duration (11.1 s)
- Outer sweep (`mag_z_low_current_points`)、Inner sweep (`rf_frequency_points`)、Expected frames (45)、Required devices (3)

Sweep dimensions 表格：sweep_id、device、type、unit、dimensions（`cartesian_grid` 3 pts × `list` 3 pts = 9 points）。

Safety summary 行：Checked steps (15)、Checked actions (0)、Info/Warning/Error (0/0/0)、Operator approval (Required)。

### Recipe
![Recipe](../../screenshots/system-scan-recipe.png)

Recipe metadata 表格：ID、Kind (`system_scan_recipe`)、Schema version (`0.2.0`)、Description、Station ref、Physical response required (`false`)。

5 个可折叠手风琴区域：
1. **Devices**: smb100a/oe1022d/magnetic/laser，含 device_id 和 required 状态
2. **Fixed params**: 完整设备配置树 — smb100a RF/FM/LF、oe1022d input/gain/filter、magnetic coil_matrix、laser
3. **Sweeps**: 2 个 sweep 定义（cartesian_grid + list）
4. **Acquisition policy**: per_final_sweep_point 模式、start_after 事件门控
5. **Safety**: no_internal_smb_sweep、no_realtime_csv 等标志

### Station Safety
![Station Safety](../../screenshots/system-scan-station-safety.png)

Station metadata: `station_nv_lab_01`、NV Lab ODMR Station 01、Schema 0.2.0。

Devices 表格（6 行）: Device ID、Kind、Transport、Address、Expected S/N、Timeout、Profile。

Safety limits: smb100a（频率范围、max_power、max_fm_deviation）、magnetic（per-axis max_current、max_ramp）。

### Device Profiles
![Device Profiles](../../screenshots/system-scan-device-profiles.png)

4 个可切换 profile 标签（smb100a / oe1022d / magnetic / laser）。

每个 profile 显示：metadata 表格（ID、device_type、Description、Schema version）+ 完整 JSON 配置树。

当前截图显示 smb100a profile（RF 2.882GHz、-30dBm、CW、FM HIGH_DEVIATION 4MHz、LF 500Hz SQUARE）。

### Resolved Steps
![Resolved Steps](../../screenshots/system-scan-resolved-steps.png)

15 步表格，分 Phase 显示（SETUP ×3、MEASURE pt_000~pt_008 ×9、CLEANUP ×3）。

列：Step ID、Phase badge（SETUP/MEASURE/CLEANUP）、Point、Sweep coordinates、Acquisition（ON/OFF badge）。

Sweep coordinates 显示完整的笛卡尔积坐标：`mag_z_low_current_points.bz_nt=-1,000` + `rf_frequency_points.frequency_hz=2,878,000,000`。

### Safety Report
![Safety Report](../../screenshots/system-scan-safety-report.png)

Decision: ALLOW 横幅 + 30 checks evaluated。

Filter 按钮：All (30)、Pass (29)、Warn (1)、Fail (0)。

Checks 表格：Status badge（PASS/WARN/FAIL）、Check name、Message、Value、Limit。

关键检查项：recipe_schema_valid、rf_frequency_within_range、smb_power_within_limit、coil_matrix_present、coil_matrix_not_singular、b_vector_within_limit、current_ramp_within_limits。

唯一 Warn：`required_reference_lock_declared`（reference lock not required by station）。

### Dry Run
![Dry Run](../../screenshots/system-scan-dry-run.png)

Summary cards：Total steps (15)、Total points (9)、Expected frames (45)、Estimated duration (11.1 s)、Outer/Inner sweep、Hazard actions (9)、Operator approval (Required)。

3 个 Phase 手风琴：
1. **SETUP**（蓝色）: preflight、RF config、magnetic baseline zero
2. **MEASURE**（紫色）: Nested sweep 3×3=9 points，带黄色 hazard badge "RF output ON at each measure point"
3. **CLEANUP**（黄色）: RF OFF、magnetic zero、LOCAL mode

## Code Structure

- **文件**: `src/routes/SystemScanPage.tsx` (921 行)
- **数据**: `src/mock-data/m5b/index.ts` 导入 6 个 JSON 文件（Vite `?inline` bundle）
  - `recipe.json` / `resolved.json` / `safety_report.json` / `dry_run_plan.json` / `station.json` / `deviceProfiles.json`
- **类型**: `src/types/m5b.ts` — 6 个 TypeScript interface（M5bRecipe / M5bResolvedRecipe / M5bSafetyReport / M5bDryRunPlan / M5bStation / M5bDeviceProfile）
- **状态**: `useState<TabKey>("overview")` 管理当前 tab
- **工具函数**:
  - `statusStyle()` / `phaseStyle()` — badge 颜色映射
  - `renderJsonTree()` — 递归渲染任意 JSON 值（含 3×3 matrix 特殊网格渲染）
- **组件**: 无外部 UI 库；纯 inline-style React
- **交互**: 无 Tauri 命令；所有 tab 切换和 accordion 均为本地状态
- **布局**: Tab 按钮栏 + 条件渲染的 tab 内容区

---

# About / Boundaries

## Route
`/about`

## Screenshot
![About / Boundaries](../../screenshots/about.png)

## Visual Description

页面展示 **GUI-M0 的能力边界声明**，纯静态文本，分为三个区域：

1. **Boundary Statement** (全宽卡片):
   - "This GUI is mock-only."
   - "It does not connect to devices."
   - "It does not call executor."
   - "It does not send SCPI."
   - "It does not read OE1022D RALL?."
   - "It does not write experiment data."
   - "Future M2 integration must go through backend APIs, executor, and safety interlock."

2. **Allowed in M0** (左半卡片，绿色标题):
   - Display mock run summary
   - Display dry-run plan
   - Display safety report
   - Display events
   - Display artifact inventory
   - Display disabled future controls

3. **Forbidden in M0** (右半卡片，红色标题):
   - serial / USB / VISA / TCP socket access
   - SCPI sending
   - executor calls
   - hardware polling
   - raw data parsing
   - run data writing
   - AI live hardware control

4. **Future M1 / M2 Integration Path** (全宽卡片):
   - M1: read-only backend APIs (mock listing、static file loading、replay timeline、chart preview)
   - M2: real backend commands (connect_device、status snapshot、run start、safe shutdown)；强调 GUI 只发 user intent，executor 拥有 run authority，safety 拥有 allow/reject 权

## Code Structure

- **文件**: `src/routes/AboutBoundariesPage.tsx` (124 行)
- **数据**: 100% 硬编码 JSX，无任何外部数据源
- **组件**: 纯 CSS-in-JS，无外部 UI 库
- **交互**: 无状态、无事件、无 Tauri 命令
- **布局**: 全宽 statement 卡片 → 2 列 grid（Allowed / Forbidden）→ 全宽 Future Path 卡片

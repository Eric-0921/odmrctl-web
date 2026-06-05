# GUI-M0 Screenshot Documentation

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
| 1 | [Dashboard](dashboard.md) | `/` | `dashboard.png` | `getRunSummary()` mock helper | ❌ 无 |
| 2 | [Devices](devices.md) | `/devices` | `devices.png` | 硬编码 `devices[]` 数组 | ❌ 无 |
| 3 | [Recipe](recipe.md) | `/recipe` | `recipe.png` | `getRecipe()` + `getRunSummary()` | ❌ 无 |
| 4 | [Dry Run](dry-run.md) | `/dry-run` | `dry-run.png` | `getDryRunPlan()` + `getDryRunSteps()` | ❌ 无 |
| 5 | [Safety](safety.md) | `/safety` | `safety.png` | `getSafetyReport()` | ❌ 无 |
| 6 | [Events](events.md) | `/events` | `events.png` | `getEvents()` + `getRunSummary()` | ❌ 无 |
| 7 | [Raw Data Preview](raw-data.md) | `/raw-data` | `raw-data.png` | `getRunManifest()` + `getRawArtifactSummary()` + `getIndexEntries()` | ❌ 无 |
| 8 | [Analysis Viewer](analysis-viewer.md) | `/analysis-viewer` | `analysis-viewer.png` | 用户选择的 M3.6 分析目录（JSON 文件） | ✅ `pick_analysis_directory` + `read_analysis_directory` |
| 9 | [Recipe Viewer](recipe-viewer.md) | `/recipe-viewer` | `recipe-viewer.png` | 默认示例 JSON / 用户选择的 recipe 文件 | ✅ `pick_recipe_file` + `read_recipe_file` |
| 10 | [System Scan](system-scan.md) | `/system-scan` | `system-scan-*.png` (7 tabs) | `src/mock-data/m5b/*.json` bundle | ❌ 无 |
| 11 | [About / Boundaries](about.md) | `/about` | `about.png` | 100% 硬编码 JSX | ❌ 无 |

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

## 文件索引

```
docs/gui-screenshots/
├── README.md                 # 本文件（汇总索引）
├── dashboard.md              # Dashboard 页面说明
├── devices.md                # Devices 页面说明
├── recipe.md                 # Recipe 页面说明
├── dry-run.md                # Dry Run 页面说明
├── safety.md                 # Safety 页面说明
├── events.md                 # Events 页面说明
├── raw-data.md               # Raw Data Preview 页面说明
├── analysis-viewer.md        # Analysis Viewer 页面说明
├── recipe-viewer.md          # Recipe Viewer 页面说明
├── system-scan.md            # System Scan 页面说明（7 tabs）
└── about.md                  # About / Boundaries 页面说明

screenshots/
├── dashboard.png
├── devices.png
├── recipe.png
├── dry-run.png
├── safety.png
├── events.png
├── raw-data.png
├── analysis-viewer.png
├── recipe-viewer.png
├── about.png
├── system-scan-overview.png
├── system-scan-recipe.png
├── system-scan-station-safety.png
├── system-scan-device-profiles.png
├── system-scan-resolved-steps.png
├── system-scan-safety-report.png
└── system-scan-dry-run.png
```

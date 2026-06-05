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

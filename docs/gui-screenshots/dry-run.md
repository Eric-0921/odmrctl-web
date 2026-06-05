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

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

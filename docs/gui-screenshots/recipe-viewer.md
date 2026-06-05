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

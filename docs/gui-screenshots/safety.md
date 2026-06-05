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

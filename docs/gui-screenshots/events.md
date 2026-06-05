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

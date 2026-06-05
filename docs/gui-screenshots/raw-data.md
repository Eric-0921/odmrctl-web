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

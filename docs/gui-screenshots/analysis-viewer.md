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

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

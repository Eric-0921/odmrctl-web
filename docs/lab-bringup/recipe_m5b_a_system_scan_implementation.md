# Recipe-M5B-A Implementation Report

> 文档定位：M5B-A 实现报告
> 创建日期：2026-05-31
> 对应 commit：TBD

---

## 1. 本里程碑的目的

P6.2 完成后，项目积累了一系列独立的 lab 工具（`rf_mag_oe_minimal_run`、`common_preflight` 等），但缺乏从 **system-level JSON recipe** 到 **resolved_recipe → safety → dry-run → executor** 的主线贯通。

M5B-A 的目标是把开发方向重新拉回到 recipe 主线：
- 定义 `system_scan_recipe` JSON 合约
- 实现最小可运行的 parser / validator / expander / safety-report
- 提供可序列化的 example JSON（recipe、resolved、safety、dry-run）
- 用 harness 测试验证核心契约

---

## 2. 与 PRD 的关系

| PRD | 本 milestone 实现的内容 |
|-----|------------------------|
| PRD-04 Recipe JSON Schema | 新增 `system_scan_recipe` kind；定义 fixed_params、sweeps、sweep_order、acquisition_policy 结构 |
| PRD-09 Magnetic Field Planner | 磁场笛卡尔网格扫描（cartesian_grid）作为 sweep 类型之一；B vector 模型；固定/变化轴语义 |
| PRD-05 Compiler & Executor | 最小 expander：`expand_system_scan_recipe()`；dry-run builder；setup + measure + cleanup 三阶段 |
| PRD-10 Safety & Interlock | 15 项安全检查；laser 禁用策略；recipe 不可覆盖 safety limit；operator approval 强制 |

---

## 3. 文件路径

| 产物 | 路径 |
|------|------|
| 合约定义 | `docs/lab-bringup/recipe_m5b_system_scan_contract.md` |
| 实现报告 | `docs/lab-bringup/recipe_m5b_a_system_scan_implementation.md` |
| 示例 recipe | `examples/recipes/m5b_rf_mag_oe_system_scan.recipe.json` |
| 示例 resolved | `examples/resolved/m5b_rf_mag_oe_system_scan.resolved.json` |
| 示例 safety | `examples/safety/m5b_rf_mag_oe_system_scan.safety_report.json` |
| 示例 dry-run | `examples/dry_run/m5b_rf_mag_oe_system_scan.dry_run_plan.json` |
| Rust 类型 + parser | `crates/odmr-recipe/src/system_scan.rs` |
| Rust expander | `crates/odmr-compiler/src/system_scan.rs` |
| Rust safety | `crates/odmr-safety/src/system_scan.rs` |
| GUI 类型 | `apps/desktop/src/types/recipe.ts` |
| GUI 验证 | `apps/desktop/src/utils/recipeValidation.ts` |
| GUI 页面 | `apps/desktop/src/routes/RecipeViewerPage.tsx` |

---

## 4. 支持的扫描类型

| 扫描类型 | 说明 | M5B-A 状态 |
|----------|------|------------|
| `cartesian_grid` | 磁场笛卡尔网格，支持 `value`（固定轴）和 `values`（变化轴） | ✅ 支持 |
| `values_list` | 标量值列表，如 RF 频率离散值 | ✅ 支持 |
| 多轴 cartesian | 多轴同时变化（Bx × By × Bz） | ❌ 仅单轴变化 |
| 球坐标扫描 | spherical_sweep | ❌ 未支持 |
| 路径扫描 | path_sweep | ❌ 未支持 |

---

## 5. 采集策略语义

`acquisition_policy` 定义：
- `mode`: `"per_final_sweep_point"` — 仅在所有 sweep 轴的最终组合点触发采集
- `start_after`: 前置条件列表（magnetic_settled, rf_configured, rf_output_on_confirmed）
- `frames_per_point`: 每点采集帧数
- `attach_device_state_snapshot`: 是否附加设备状态快照

展开后，每个 `measure` step 的 `acquisition.enabled = true`，setup/cleanup 为 `false`。

---

## 6. 可追溯性语义

每个 resolved measure step 包含：
```json
{
  "traceability": {
    "source_recipe_id": "m5b_rf_mag_oe_system_scan",
    "required_state_snapshot": true,
    "required_step_hash": true
  }
}
```

确保每个数据点可回溯到：
- `run_id` → `step_id` → `sweep_coordinates` → `source_recipe_id` → `source_recipe_hash`

---

## 7. 已实现的 Rust API

### odmr-recipe
```rust
pub fn parse_system_scan_recipe(json: &str) -> Result<SystemScanRecipe, RecipeError>
pub fn load_system_scan_recipe(path: &Path) -> Result<SystemScanRecipe, RecipeError>
pub fn validate_system_scan_recipe(recipe: &SystemScanRecipe) -> Result<(), ValidationError>
pub fn compute_system_scan_hash(recipe: &SystemScanRecipe) -> String
```

### odmr-compiler
```rust
pub fn expand_system_scan_recipe(recipe: &SystemScanRecipe) -> Result<ResolvedSystemScan, SystemScanCompileError>
pub fn build_system_scan_dry_run(recipe: &SystemScanRecipe, resolved: &ResolvedSystemScan) -> SystemDryRunPlan
```

### odmr-safety
```rust
pub fn build_system_scan_safety_report(recipe: &SystemScanRecipe, resolved: &ResolvedSystemScan) -> SystemSafetyReport
```

---

## 8. 测试覆盖

### odmr-recipe (system_scan 模块): 8 测试
- example_system_scan_recipe_parses
- magnetic_sweep_has_three_points
- rf_sweep_has_three_points
- laser_enabled_is_rejected
- command_array_in_fixed_params_is_rejected
- safety_limit_override_is_rejected
- missing_sweep_in_order_is_rejected
- empty_values_list_is_rejected

### odmr-compiler (system_scan 模块): 11 测试
- expansion_produces_nine_measure_steps
- first_point_has_correct_coordinates
- last_point_has_correct_coordinates
- measure_steps_have_acquisition_enabled
- setup_and_cleanup_have_acquisition_disabled
- every_measure_step_has_traceability_snapshot_required
- resolved_has_stable_step_ids
- magnetic_target_state_is_correct
- rf_target_state_is_correct
- dry_run_has_correct_summary
- expansion_with_zero_sweeps_returns_empty_measure

### odmr-safety (system_scan 模块): 5 测试
- safety_report_allows_example_recipe
- safety_report_requires_operator_approval
- all_expected_checks_present
- laser_enabled_is_rejected
- safety_limit_override_is_rejected
- report_serializes_to_json

**总计：24 个新增测试全部通过。**

---

## 9. GUI 兼容性

最小更新：
- `recipe.ts` 新增 `SystemScanRecipe` 接口
- `recipeValidation.ts` 识别 `system_scan_recipe` kind，返回 valid-but-not-preview 结果
- `RecipeViewerPage.tsx` 显示 "system_scan_recipe recognized — full GUI preview not yet implemented"

未实现（M5B-GUI 里程碑）：
- 固定参数展示
- sweep 计数与展开预览
- 预计点数/帧数实时计算
- resolved/dry-run/safety 面板

---

## 10. P6.2 真实回归状态

**不属于本 milestone 范围。** P6.2 的 real hardware regression 是独立验证项，与 M5B-A（纯软件层 recipe 基础设施）无关。本 milestone 的所有验收项（parser / compiler / safety / example JSON）均已通过 harness/mock 测试完成，符合项目 mock-first 开发规范。

---

## 11. 限制与未来工作

| 限制 | 说明 | 后续里程碑 |
|------|------|------------|
| 磁场仅支持单轴变化 | cartesian_grid 中仅允许一个 axis 为 `values`，其余为 `value` | M5B-B |
| 无 coil matrix 反演 | B field → current 使用固定比例（未接入 `odmr-mag` coil matrix） | M5B-B |
| 激光器强制禁用 | safety 明确拒绝 `enabled: true` | Laser-M4 |
| 无 block/profile 引用 | fixed_params 直接内联 | M5B-C |
| 无 GUI 完整预览 | 仅识别 kind，显示 placeholder | M5B-GUI |
| 未连接真实 executor | resolved recipe 仅为 JSON 产物，未被 executor 消费 | M4.2 |
| 无运行时 safety gate | `build_system_scan_safety_report` 为静态检查 | M4.2 |

---

## 12. 诚实开发笔记

### 编译错误
1. `include_str!` 路径错误：从 `crates/odmr-recipe/src/` 到 `examples/recipes/` 应为 `../../../` 而非 `../../../../`。
2. `odmr-compiler` 缺少 `sha2`/`hex`：通过在 `odmr-recipe` 中暴露 `compute_system_scan_hash()` 解决，避免向 compiler 引入新依赖。
3. `odmr-safety` 无法引用 `odmr-compiler::ResolvedSystemScan`：向 `odmr-safety/Cargo.toml` 添加 `odmr-compiler` 依赖（Layer 2 → Layer 2，符合架构）。

### Schema 歧义
- RF sweep 的 `type` 字段：任务示例中 `cartesian_grid` 有 `"type": "cartesian_grid"`，但 `values_list` 无 `type`。改为 `#[serde(untagged)]` 使两种形式均可解析。
- `sweep_coordinates` 的 key：应使用 `sweep_id` + 短轴名（如 `rf_frequency_points.frequency_hz`），而非完整 `axis` 路径。

### 修复的 bug
- 首次展开时 `sweep_coordinates` 使用了完整 `axis` 路径作为 key，导致测试期望的短 key 不匹配。
- `recipe_integration_tests.rs` 尝试将所有 `examples/recipes/*.json` 作为 `Recipe` 解析，新增的 `system_scan_recipe` 导致失败。更新测试以按 `kind` 分派解析器。

### 未修复的问题
- 无

### 临时简化
- `estimated_duration_s` 使用启发式计算（settle_ms + frames × inter_frame_delay + 固定开销），非精确模型。
- safety report 的 `magnetic_current_within_limit` 使用 B field 到 current 的固定比例估算，未接入真实 coil matrix。

# odmr-compiler

**Layer 2** — Recipe 编译管线。

## 职责

- 接收 `recipe.json`，输出 `resolved_recipe.json`
- 参数展开（变量、循环、sweep 展开）
- Step 依赖解析与拓扑排序
- Timing 计算
- Dry-run 计划生成

## 依赖

- `odmr-recipe`
- `odmr-safety`
- `odmr-types`

## 参考

- `docs/prd/05_recipe_compiler_executor_prd_v0.2.md`

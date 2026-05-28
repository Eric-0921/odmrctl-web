# odmr-recipe

**Layer 2** — Recipe 数据结构定义与 JSON Schema 验证。

## 职责

- Recipe JSON 反序列化/序列化
- JSON Schema 验证
- Recipe 结构遍历、参数展开
- Recipe 版本兼容性

## 依赖

- `odmr-types`

## 不负责

- Recipe 编译/展开（那是 odmr-compiler 的事）
- 安全策略校验（那是 odmr-safety 的事）

## 参考

- `docs/prd/04_recipe_json_schema_prd_v0.2.md`
- `schemas/`

# odmr-executor

**Layer 3** — Recipe 执行引擎。

## 职责

- 执行状态机（Idle → Running → Paused → Stopped → Error）
- Step 调度与时序控制
- 设备命令编排（通过 odmr-device trait）
- 实时采集协调（OE1022D 数据流管理）
- 急停响应

## 依赖

- `odmr-device`（通过 trait）
- `odmr-recipe`
- `odmr-safety`
- `odmr-logging`

## 不负责

- GUI 控制逻辑
- 数据格式转换

## 参考

- `docs/prd/05_recipe_compiler_executor_prd_v0.2.md`
- `docs/prd/06_timing_sync_averaging_prd_v0.2.md`

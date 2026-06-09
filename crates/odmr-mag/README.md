# odmr-mag

**Layer 1** — 三轴磁场控制模块（Maynuo M8812 协议规划层）。

## 职责

- Maynuo M8812 SCPI 协议封装与磁场规划
- 线圈矩阵建模与磁场-电流转换
- 磁场安全策略（电流限制、Ramp 速率限制、Settle 时间）
- 零点锁定（zero-lock）工作流
- 顺序多轴运行（sequential axis run）命令计划
- Mock 磁场轴状态机与测试 fixture

## 依赖

- `odmr-device`
- `odmr-types`
- `odmr-maynuo-m8812`

## 状态

**M5A 真实硬件已验证** — 已通过 `rf_mag_oe_minimal_run` 在真实 Maynuo M8812 + 线圈上验证单轴及顺序多轴磁场控制。

## 参考

- `docs/prd/` — magnetic coil control workflow, measured coil constants
- `tools/lab/maynuo_m8812_*` — 各阶段 bring-up 工具

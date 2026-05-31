# odmr-mag

**Layer 1** — 三轴磁场控制模块（Maynuo M8812 协议）。

## 职责

- Maynuo M8812 SCPI 协议封装
- 线圈矩阵建模与磁场-电流转换
- 磁场安全策略（电流限制、Ramp 速率限制、Settle 时间）
- Mock 磁场轴状态机与 Replay

## 依赖

- `odmr-device`
- `odmr-types`

## 参考

- `docs/adr/ADR-008-magnetic-output-safety-boundary.md`
- `docs/prd/09_magnetic_field_planner_prd_v0.2.md`

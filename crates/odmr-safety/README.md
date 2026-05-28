# odmr-safety

**Layer 2** — 安全互锁策略引擎。

## 职责

- `SafetyPolicy` trait
- `InterlockEngine`：参数边界检查、急停触发条件
- 硬件保护限制（频率上限、功率上限、温度阈值）
- Safety report 生成

## 依赖

- `odmr-types`

## 不负责

- GUI 层安全展示（那是 apps/desktop 的事）
- 硬件急停按钮驱动（那是 executor + device driver 的事）

## 参考

- `docs/prd/10_safety_interlock_prd_v0.2.md`

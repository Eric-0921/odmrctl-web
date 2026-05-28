# odmr-types

**Layer 0** — 全仓库共享的基础类型。零依赖（std only）。

## 职责

- DeviceId, RecipeStep, Event, RunId, Timestamp
- 错误类型枚举
- 基础 trait（无外部依赖的部分）

## 不负责

- 任何 I/O 操作
- 设备协议细节
- 序列化/反序列化格式选择
- 具体数值常量

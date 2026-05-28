# AGENTS.md

ODMR 自动化采集系统——NV 色心 ODMR 实验的 Tauri 桌面控制平台。Rust 后端负责设备编排、recipe 执行、高频采集与数据落盘；前端仅做交互展示；Python 仅离线分析与 recipe 生成。

## 仓库导航

| 路径 | 内容 | 何时查阅 |
|------|------|----------|
| `docs/prd/` | 12 份产品需求（总纲 + 子模块） | 理解功能边界与业务意图 |
| `docs/adr/` | 5 份架构决策记录 | 理解技术选型的原因 |
| `docs/architecture/ARCHITECTURE.md` | 分层模型、crate 职责、依赖方向 | 写代码前的入口 |
| `docs/decisions/` | 进行中的设计决策 | 开发中遇到选择时写这里 |
| `crates/` | Rust workspace 各 crate | 每个 crate 的 README 定义其边界 |
| `apps/desktop/` | Tauri 桌面应用 | GUI 入口 |
| `python/` | recipe 工具 + 离线分析 | 不参与实时链路 |
| `schemas/` | recipe JSON Schema、数据格式规范 | 验证 recipe 输入 |
| `examples/` | 示例 recipe 文件 | 参考用法 |
| `tests/` | 集成测试（fixtures + golden） | mock-first 开发 |

从 `docs/architecture/ARCHITECTURE.md` 开始，它指向各 crate 和对应 PRD。

## 架构硬约束

这些是不可协商的规则，在 `docs/architecture/ARCHITECTURE.md` 中详细定义：

1. **依赖方向固定**：Layer 0 (设备) → Layer 1 (驱动) → Layer 2 (domain) → Layer 3 (runtime) → Layer 4 (API) → Layer 5 (GUI)。禁止反向依赖。
2. **单一硬件入口**：所有硬件访问必须经过 DeviceManager → ResourceLease → Driver。GUI、AI、Python 禁止直接访问硬件。
3. **实时链路隔离**：采集线程只做 read/timestamp/parse/buffer/write。CSV 导出、拟合、分析不能进入实时链路。
4. **Recipe 驱动**：实验执行必须来自 recipe.json → compiler → safety → dry-run → 人工批准 → executor。AI 不能直接发送硬件命令。
5. **Raw-first 数据**：实时阶段只写 raw bin + index.jsonl + events.jsonl。实验后再生成 parquet/csv。

## 机械化检查

`scripts/check-consistency.sh` 守卫结构漂移：

- **C1**：`crates/` 下的目录与根 `Cargo.toml` workspace members 一致
- **C2**：`docs/prd/*.md` 编号连续
- **C3**：PRD 数量与 README 声明一致
- **C4**：ADR 数量与 README 声明一致
- **C5**：每个 crate 目录包含 README.md

执行：`bash scripts/check-consistency.sh`（仓库根目录）
启用 pre-commit hook：`git config core.hooksPath .githooks`

## 开发流程

1. 新功能先写 `docs/decisions/` 中的设计决策
2. 对照 `docs/architecture/ARCHITECTURE.md` 确认 crate 边界
3. mock-first：设备交互先在 `odmr-harness` 中用 fake device 验证
4. 集成测试 fixture 放 `tests/fixtures/`，golden 数据放 `tests/golden/`
5. recipe 示例放 `examples/`，schemas 变更同步更新 `schemas/`

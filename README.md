# odmrctl-web

ODMR（Optically Detected Magnetic Resonance）自动化采集平台。Tauri 桌面应用 + Rust 高性能后端，面向 NV 色心 ODMR 实验的设备编排、recipe 执行、高频采集、数据落盘与离线分析。

> **人类设计约束，智能体执行。**

## 架构快照

```
Layer 6: Python Offline    python/analysis/  python/recipe_tools/
Layer 5: GUI               apps/desktop/
Layer 4: Application API   Tauri commands
Layer 3: Runtime           odmr-executor  odmr-logging  odmr-replay  odmr-harness
Layer 2: Domain            odmr-recipe  odmr-compiler  odmr-safety  odmr-config
Layer 1: Drivers           odmr-smb100a  odmr-oe1022d  odmr-device
Layer 0: Types             odmr-types
```

## 核心理念

| 原则 | 含义 |
|------|------|
| 单一硬件入口 | 所有硬件访问经过 DeviceManager → ResourceLease → Driver |
| 实时链路隔离 | 采集线程只做 read/parse/buffer/write，CSV/分析/图表离线 |
| Recipe 驱动 | 实验执行 = recipe.json → compile → safety → dry-run → 批准 → run |
| Raw-first 数据 | 实时写 raw bin，实验后转 parquet/csv |
| Mock-first 开发 | 新功能先在 harness 中用 fake device 验证 |

## 文档索引

| 目录 | 内容 | 数量 |
|------|------|------|
| `docs/prd/` | 产品需求文档 | 13 份 |
| `docs/adr/` | 架构决策记录 | 5 份 |
| `docs/architecture/` | 架构总图、分层规则 | — |
| `docs/decisions/` | 进行中的设计决策 | — |
| `schemas/` | Recipe JSON Schema | — |
| `examples/` | 示例 recipe | — |

## 开发入口

- 新人/智能体：从 `AGENTS.md` 开始
- 理解架构：`docs/architecture/ARCHITECTURE.md`
- 技术选型原因：`docs/adr/ADR-*.md`
- 功能边界细节：`docs/prd/0*_prd_v0.2.md`

## 机械化检查

```bash
bash scripts/check-consistency.sh
git config core.hooksPath .githooks   # 启用 pre-commit
```

## 技术栈

Rust workspace（12 crates）+ Tauri + Web 前端 + Python 离线分析

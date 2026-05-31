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
Layer 1: Drivers           odmr-smb100a  odmr-oe1022d  odmr-device  odmr-mag
Layer 0: Types             odmr-types

Lab Bringup Tools          tools/discover/  tools/lab/snapshot/  tools/manual_command_verify/
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
| `docs/adr/` | 架构决策记录 | 8 份 |
| `tools/` | 实验室联调工具 | 见下方 |
| `docs/architecture/` | 架构总图、分层规则 | — |
| `docs/decisions/` | 进行中的设计决策 | — |
| `schemas/` | Recipe JSON Schema | — |
| `examples/` | 示例 recipe | — |

## Lab Bringup 工具（M2–M3 阶段）

| 工具 | 路径 | 用途 |
|------|------|------|
| 硬件发现 | `tools/discover/` | macOS 自动扫描 LAN / Serial / USB 设备，生成 station fingerprint |
| 只读快照 | `tools/lab/snapshot/` | 连接真实 SMB100A（TCP）和 OE1022D（Serial），执行安全只读查询，生成 Markdown + JSONL 快照报告 |
| 命令验证 | `tools/manual_command_verify/` | Human-in-the-loop 验证协议：人工审批 → 执行 → 记录 JSONL 回执 |
| **SMB100A M3 工具** | | |
| 飞行前清空错误队列 | `tools/lab/smb100a_preflight_clearance/` | 仅查询模式，验证 SMB100A 处于干净、安全、RF 关闭状态 |
| RF 开关微测试 | `tools/lab/smb100a_rf_microtest/` | 受控 RF 开关，固定频率、低功率、无调制，需操作员审批 |
| FM/MOD 微测试 | `tools/lab/smb100a_fm_mod_microtest/` | 受控 FM:STAT ON / MOD:STAT ON / OUTP ON 序列，需操作员审批 |
| 安全设置 | `tools/lab/smb100a_safe_set/` | 通过允许列表验证的 SMB100A 单一 SCPI 设置命令 |
| **OE1022D M2 工具** | | |
| 采集 | `tools/lab/oe1022d_acquire/` | OE1022D 基础采集 |
| 日志采集 | `tools/lab/oe1022d_logged_acquire/` | OE1022D 采集，含结构化日志 |
| RALL 捕获 | `tools/lab/oe1022d_rall_capture/` | 全寄存器捕获 + 采集 |
| 运行审计 | `tools/lab/oe1022d_run_audit/` | OE1022D 采集运行审计 |
| 虚假桥接 | `tools/lab/oe1022d_smb_fake_bridge/` | OE1022D ↔ 虚假 SMB100A 桥接 |
| 查询桥接 | `tools/lab/oe1022d_smb_query_bridge/` | OE1022D ↔ 真实 SMB100A 查询桥接 |
| **其他** | | |
| 执行器影子运行 | `tools/lab/executor_shadow_run/` | 执行器影子运行（不接触硬件）|

> ⚠️ **ADR-004 约束**：所有 lab 工具均为只读或 human-in-the-loop 模式；AI 禁止直接控制硬件输出。

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

## GUI-M0 Mock Viewer

`apps/desktop/` 是 GUI-M0 的 mock-only Tauri + React 前端。它展示静态 bundl 的 mock 实验数据，不做任何硬件访问。

### 运行 GUI-M0

```bash
cd apps/desktop
pnpm install
pnpm tauri dev        # 开发模式，热重载
# 或
pnpm tauri build      # 发布构建
```

### GUI-M0 已知限制

- **Mock-only**：所有数据来自构建时 bundl 的静态 snapshot，运行时无 fs/fetch
- **无硬件访问**：无 serial / USB / VISA / TCP / SCPI 代码
- **无 executor 连接**：Start Run / Pause / Stop / Emergency Stop 等按钮全部禁用
- **无 rawbin 解析**：Raw Data Preview 仅展示元数据，二进制解析由 Rust 后端负责
- **无实时订阅**：Events 页面展示静态事件日志，无 WebSocket / SSE

## 技术栈

Rust workspace（13 crates）+ Tauri + Web 前端 + Python 离线分析

# odmrctl-web

ODMR（Optically Detected Magnetic Resonance）自动化采集平台。Tauri 桌面应用 + Rust 高性能后端，面向 NV 色心 ODMR 实验的设备编排、recipe 执行、高频采集、数据落盘与离线分析。

> **人类设计约束，智能体执行。**

## 架构快照

```
Layer 6: Python Offline    python/analysis/  python/recipe_tools/
Layer 5: GUI               apps/desktop/     (Tauri/Web)
Layer 4: Application API   Tauri commands    (apps/desktop/src-tauri/)
Layer 3: Runtime           odmr-executor  odmr-logging  odmr-replay  odmr-harness  odmr-live-server
Layer 2: Domain            odmr-recipe  odmr-compiler  odmr-safety  odmr-config  odmr-preflight
Layer 1: Drivers           odmr-smb100a  odmr-oe1022d  odmr-mag  odmr-maynuo-m8812  odmr-laser  odmr-device
Layer 0: Types             odmr-types

Lab Bringup Tools          tools/lab/common_preflight/  tools/lab/visa_probe/
```

当前状态：**M5C-A Device Workbench V1 已激活（Station Workbench + 4×Minimal Device Panels + Experiment Planning）；M5B System Scan recipe + Artifact Viewer 已实现；M5A RF + Mag + OE 最小组合实验已真实硬件验证；P6 common_preflight / StationLedger / DeviceLock 已固化；odmr-config / odmr-replay 已从占位转正；odmr-laser 新驱动已就位；next: M5B 多磁轴 ODMR 实采、M4.2 GUI run launcher。**

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
| `docs/decisions/` | 进行中的设计决策 | — |
| `tools/` | 实验室联调工具 | 见下方 |
| `docs/architecture/` | 架构总图、分层规则 | — |
| `schemas/` | Recipe JSON Schema | — |
| `examples/` | 示例 recipe 与设备指令 JSON | — |

## Lab Bringup 工具

### 统一 Station Preflight

| 工具 | 路径 | 用途 |
|------|------|------|
| **common-preflight** | `tools/lab/common_preflight/` | 统一站级设备预检：自动发现 + 身份验证 + 安全状态 + 设备锁 + StationLedger |

### SMB100A 工具

| 工具 | 路径 | 阶段 |
|------|------|------|
| 飞行前清空错误队列 | `tools/lab/smb100a_preflight_clearance/` | M3.0-A |
| RF 开关微测试 | `tools/lab/smb100a_rf_microtest/` | M3.0-B |
| FM/MOD 微测试 | `tools/lab/smb100a_fm_mod_microtest/` | M3.1 |
| 安全设置 | `tools/lab/smb100a_safe_set/` | M3 |
| 步进 sweep | `tools/lab/smb100a_oe1022d_step_sweep/` | M3.2 |
| 扩展 sweep | `tools/lab/smb100a_oe1022d_extended_sweep/` | M3.2 |
| VISA A/B 基准 | `tools/lab/visa_probe/` | 连接层 |

### OE1022D 工具

| 工具 | 路径 | 阶段 |
|------|------|------|
| 基础采集 | `tools/lab/oe1022d_acquire/` | M2 |
| 日志采集 | `tools/lab/oe1022d_logged_acquire/` | M2 |
| RALL 捕获 | `tools/lab/oe1022d_rall_capture/` | M2 |
| Buffer probe / 稳定性测试 | `tools/lab/oe1022d_buffer_probe/` | M5B |
| 运行审计 | `tools/lab/oe1022d_run_audit/` | M2 |
| SMB 桥接 | `tools/lab/oe1022d_smb_fake_bridge/` `oe1022d_smb_query_bridge/` | M2 |

### 磁场工具

| 工具 | 路径 | 阶段 |
|------|------|------|
| M8812 身份探针 | `tools/lab/maynuo_m8812_identity_probe/` | Mag-M2A |
| Zero baseline | `tools/lab/maynuo_m8812_zero_baseline/` | Mag-M2B |
| Recur microtest | `tools/lab/maynuo_m8812_recur_microtest/` | Mag-M3 |
| Sequential axis run | `tools/lab/maynuo_m8812_sequential_axis_run/` | Mag-M4 |

### 组合实验

| 工具 | 路径 | 阶段 |
|------|------|------|
| **RF + Mag + OE 最小组合** | `tools/lab/rf_mag_oe_minimal_run/` | **Mag-M5A** ✅ |
| Recipe 双设备运行 | `tools/lab/recipe_two_device_run/` | M3 |
| 执行器影子运行 | `tools/lab/executor_shadow_run/` | M2 |

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

## 开发策略备注

- 近期实机联调采用 `mock-first -> query-only -> operator-approved micro-test -> dual-device sweep -> recipe-shaped run -> combined run` 的渐进策略。
- `smb100a_fm_mod_microtest` 曾一度把大量流程、CLI、传输、安全和产物逻辑堆在单个 `main.rs` 中，后续已模块化拆分，`main.rs` 仅保留薄入口。后续 lab 工具默认沿用这种模块边界。

## M5C-A Device Workbench

`apps/desktop/` 是 M5C-A Device Workbench — Tauri v2 + React + Vite 桌面应用。已从 GUI-M0 的 mock-only viewer 演进为支持真实设备面板的实验工作台。

### 功能概览

- **Station Workbench**：加载 station profile，运行统一 preflight，查看预检报告
- **设备面板**（4 个最小面板）：
  - **SMB100A**：频率、功率、输出状态、调制状态 typed get/set
  - **OE1022D**：参考频率、灵敏度、时间常数、滤波斜率、输入源 typed get/set
  - **Magnetic**：单轴电流、输出状态、zero-lock、sequential run
  - **Laser**：off-only preflight 集成、功率状态 readback
- **Recipe  viewer**：M4.1 dry-run plan 可视化
- **System Scan Artifact Viewer**：M5B-B 7-tab 只读产物查看
- **Experiment Planning**：场网格扫描 recipe 生成（1D/2D/3D）
- **实时图表**：`odmr-live-server` sidecar HTTP 演示（Phase 1）
- **运行回放**：`odmr-replay` 集成 — canonical run 目录回放、legacy rawbin 迁移

### 运行 Device Workbench

```bash
cd apps/desktop
pnpm install
pnpm tauri dev        # 开发模式，热重载，端口 1420
# 或
pnpm tauri build      # 发布构建
```

### 边界声明

- **前端仍禁止直接硬件访问**：无 serial / USB / VISA / TCP / SCPI 代码
- 所有设备交互通过**类型化 Tauri Command API** 由后端执行
- 后端通过 `odmr-preflight` 和专用 driver crate 执行 discover/identify/lock/get/set
- 所有 set 操作均带 safety gate

## 技术栈

Rust workspace（17 crates）+ Tauri v2 + Web 前端 + Python 离线分析

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
Layer 1: Drivers           odmr-smb100a  odmr-oe1022d  odmr-mag  odmr-maynuo-m8812  odmr-device
Layer 0: Types             odmr-types

Lab Bringup Tools          tools/lab/common_preflight/  tools/lab/cni_laser_*/  tools/lab/visa_probe/
```

当前状态：**M5A 完成（RF + Mag + OE 最小组合实验已真实硬件验证）；连接层进入 P6 固化阶段；四类设备 auto-discovery 已打通；StationLedger 已接入；CNI Laser M2/M3 路线已建立；next: P6 preflight hardening + M5B 多磁轴 ODMR 采集。**

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

### CNI 激光器工具

| 工具 | 路径 | 阶段 |
|------|------|------|
| Fake driver + 协议 | `tools/lab/cni_laser_fake_driver/` | Laser-M1 |
| Off-only preflight | `common_preflight` 集成 | Laser-M2 |
| 低功率微测试 | `tools/lab/cni_laser_microtest/` | Laser-M3 |

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

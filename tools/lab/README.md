# Lab Bringup Tools

ODMR 实验室联调工具集，覆盖 M2（硬件发现）和 M3（SMB100A/OE1022D 受控操作）阶段。

## 1. 工具清单

| # | 工具 | 路径 | 类型 | 阶段 |
|---|------|------|------|------|
| 1 | 硬件发现 | `tools/discover/` | 扫描 | M2 |
| 2 | 只读快照 | `tools/lab/snapshot/` | 查询 | M2 |
| 3 | 命令验证 | `tools/manual_command_verify/` | 人工审批 | M2 |
| 4 | SMB100A 飞行前清空 | `smb100a_preflight_clearance/` | 查询+诊断 | M3.0-A |
| 5 | SMB100A RF 微测试 | `smb100a_rf_microtest/` | 受控设置 | M3.0-B |
| 6 | SMB100A FM/MOD 微测试 | `smb100a_fm_mod_microtest/` | 受控设置 | M3.1 |
| 7 | SMB100A 安全设置 | `smb100a_safe_set/` | 受控设置 | M3 |
| 8 | OE1022D 采集 | `oe1022d_acquire/` | 采集 | M2 |
| 9 | OE1022D 日志采集 | `oe1022d_logged_acquire/` | 采集 | M2 |
| 10 | OE1022D RALL 捕获 | `oe1022d_rall_capture/` | 采集 | M2 |
| 11 | OE1022D 运行审计 | `oe1022d_run_audit/` | 采集 | M2 |
| 12 | OE1022D-SMB 桥接 | `oe1022d_smb_fake_bridge/` `oe1022d_smb_query_bridge/` | 桥接 | M2 |
| 13 | 执行器影子运行 | `executor_shadow_run/` | 模拟 | M2 |

## 2. 跨工具重复代码

以下组件在多个工具中以近乎相同的副本存在：

| 组件 | 工具 | 行数 |
|------|------|------|
| `SmbTransport` (TCP) | preflight, rf_microtest, fm_mod_microtest | ~45 行 × 3 |
| `do_smb_query` | preflight, rf_microtest, fm_mod_microtest | ~50 行 × 3 |
| `do_smb_set` | rf_microtest, fm_mod_microtest | ~70 行 × 2 |
| `CommandAuditEntry` (struct) | preflight, rf_microtest, fm_mod_microtest | ~15 行 × 3 |
| `SMB_QUERY_ALLOWLIST` | preflight, rf_microtest, fm_mod_microtest | ~15 行 × 3 |
| `sha256_bytes` / `sha256_file` | preflight, rf_microtest, fm_mod_microtest | ~15 行 × 3 |
| `write_jsonl` | preflight, rf_microtest, fm_mod_microtest | ~15 行 × 3 |
| `validate_smb_query_only` | preflight, rf_microtest, fm_mod_microtest | ~25 行 × 3 |
| `validate_microtest_set_command` | rf_microtest, fm_mod_microtest | ~20 行 × 2 |
| `is_forbidden_command` | preflight, rf_microtest, fm_mod_microtest | ~10 行 × 3 |
| `classify_command_for_audit` | preflight, rf_microtest, fm_mod_microtest | ~10 行 × 3 |
| `is_safety_relevant` | preflight, rf_microtest, fm_mod_microtest | ~10 行 × 3 |
| `utc_now_ms` / wall time helpers | preflight, rf_microtest | ~40 行 × 2 |
| `make_event` | preflight, rf_microtest | ~20 行 × 2 |
| `TimelineTracker` | preflight, rf_microtest | ~30 行 × 2 |
| SCPI semicolon defense | rf_microtest (M3.1.2), fm_mod_microtest | ~6 行 × 2 |
| LF shape validation | fm_mod_microtest (only) | ~15 行 × 1 |

**合计估计**：~400 行在 3 个 SMB100A 工具中重复。

## 3. M3.2 可复用模块

`smb100a_fm_mod_microtest` 曾在早期把 CLI、传输、安全、时序、产物写入和测试编排堆进单个 `main.rs`，规模一度接近 3000 行。后续已拆分为薄入口 + 模块目录。

从 `smb100a_fm_mod_microtest/src/` 来看，已模块化的部分中：

| 模块 | 可复用性 | 说明 |
|------|----------|------|
| `transport.rs` | 高 | 可直接复制到 M3.2 sweep 工具 |
| `safety.rs` | 高 | allowlists + 验证函数可直接复制，需扩展 sweep 命令 |
| `shutdown.rs` | 高 | 紧急停机逻辑可直接复制 |
| `types.rs` | 中 | 结构体可作为模板，需按 M3.2 产物调整 |
| `app.rs` | 中 | 文件写入/制品产出的编排框架可复用 |
| `artifacts.rs` | 高 | sha256/write_jsonl 工具函数可直接复制 |
| `timeline.rs` | 高 | 时间/事件跟踪可直接复制 |
| `cli.rs` | 低 | 每个工具的 CLI 参数不同 |
| `sequence.rs` | 低 | 核心序列逻辑需重写为 sweep 版本 |
| `main.rs` | 低 | 工具入口，每个工具独立 |

## 4. M3.2 后提取计划

目标：将共享组件提取为共享 crate，消除重复。

### Phase 1: 提取共享 crate（低风险，纯代码移动）

- **`odmr-smb100a`**：扩展现有的 `crates/odmr-smb100a/`，添加：
  - `SmbTransport`（从 M3.1 transport.rs）
  - `do_smb_query` / `do_smb_set`（从 M3.1 sequence.rs 拆分）
  - 通用 SCPI 工具函数
- **`odmr-types`**：扩展，添加：
  - `CommandAuditEntry`
  - `SmbQueryResult`
  - 通用产物 struct 的 trait 约束

### Phase 2: M3 工具迁移（中等风险）

- M3.1, M3.0-B, M3.0-A 逐个切换到共享 crate
- 每个切换后运行全量测试验证行为不变
- 删除各自的本地副本

### Phase 3: OE1022D 工具审计（低优先级）

- OE1022D 工具的重复模式扫描
- 考虑提取 `odmr-oe1022d` 共享传输层

## 安全约束

- 所有 lab 工具均为只读或 human-in-the-loop 模式（ADR-004）
- AI 禁止直接控制硬件输出
- SCPI 命令只通过硬编码 allowlist，默认拒绝一切未列入的命令
- 所有提交前需通过 `scripts/check-consistency.sh` 和 `cargo clippy --workspace`

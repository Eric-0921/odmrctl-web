# Lab Bringup Tools

ODMR 实验室联调工具集，覆盖 M2（硬件发现）到 M5A（RF + Mag + OE 最小组合实验）阶段。

## 1. 统一 Station Preflight

| 工具 | 路径 | 类型 | 说明 |
|------|------|------|------|
| **common-preflight** | `common_preflight/` | 统一预检 | 自动发现 + 身份验证 + 安全状态 + 设备锁 + StationLedger |

## 2. 工具清单

### SMB100A

| # | 工具 | 路径 | 类型 | 阶段 |
|---|------|------|------|------|
| 1 | 飞行前清空 | `smb100a_preflight_clearance/` | 查询+诊断 | M3.0-A |
| 2 | RF 微测试 | `smb100a_rf_microtest/` | 受控设置 | M3.0-B |
| 3 | FM/MOD 微测试 | `smb100a_fm_mod_microtest/` | 受控设置 | M3.1 |
| 4 | 安全设置 | `smb100a_safe_set/` | 受控设置 | M3 |
| 5 | 步进 sweep | `smb100a_oe1022d_step_sweep/` | 扫描 | M3.2 |
| 6 | 扩展 sweep | `smb100a_oe1022d_extended_sweep/` | 扫描 | M3.2 |
| 7 | VISA A/B 基准 | `visa_probe/` | 连接层 | 性能评估 |

### OE1022D

| # | 工具 | 路径 | 类型 | 阶段 |
|---|------|------|------|------|
| 8 | 基础采集 | `oe1022d_acquire/` | 采集 | M2 |
| 9 | 日志采集 | `oe1022d_logged_acquire/` | 采集 | M2 |
| 10 | RALL 捕获 | `oe1022d_rall_capture/` | 采集 | M2 |
| 11 | 运行审计 | `oe1022d_run_audit/` | 采集 | M2 |
| 12 | SMB 桥接 | `oe1022d_smb_fake_bridge/` `oe1022d_smb_query_bridge/` | 桥接 | M2 |

### 磁场 (Maynuo M8812)

| # | 工具 | 路径 | 类型 | 阶段 |
|---|------|------|------|------|
| 13 | 身份探针 | `maynuo_m8812_identity_probe/` | 查询 | Mag-M2A |
| 14 | Zero baseline | `maynuo_m8812_zero_baseline/` | 受控设置 | Mag-M2B |
| 15 | Recur microtest | `maynuo_m8812_recur_microtest/` | 受控设置 | Mag-M3 |
| 16 | Sequential axis | `maynuo_m8812_sequential_axis_run/` | 受控设置 | Mag-M4 |

### 组合实验

| # | 工具 | 路径 | 类型 | 阶段 |
|---|------|------|------|------|
| 17 | **RF + Mag + OE** | `rf_mag_oe_minimal_run/` | 组合 | **Mag-M5A** ✅ |
| 18 | Recipe 双设备 | `recipe_two_device_run/` | 组合 | M3 |
| 19 | 执行器影子 | `executor_shadow_run/` | 模拟 | M2 |

### CNI 激光器

| # | 工具 | 路径 | 类型 | 阶段 |
|---|------|------|------|------|
| 20 | Fake driver | `cni_laser_fake_driver/` | 协议验证 | Laser-M1 |
| 21 | 微测试 | `cni_laser_microtest/` | 受控设置 | Laser-M3 |

## 3. 跨工具共享组件

`common_preflight` 已统一以下能力，取代各工具独立的连接逻辑：

| 组件 | 覆盖设备 | 功能 |
|------|----------|------|
| `smb_probe` | SMB100A | TCP 连接 + SCPI IDN + 错误队列 + 安全状态 + 子网扫描 |
| `oe_probe` | OE1022D | 串口枚举 + `*IDN?` 匹配 + 安全状态 |
| `maynuo_probe` | Maynuo M8812 | 串口枚举 + SN 匹配 + 电流读回 + safe_zero_and_local |
| `cni_laser_probe` | CNI Laser | 串口枚举 + 帧 echo 识别 + laser_off 安全确认 |
| `device_lock` | 全部 | 跨进程设备锁（flock） |
| `station_report` | 全部 | JSON/Markdown 报告生成 |
| `ledger` | 全部 | Station 状态持久化 |

## 4. M5B 后提取计划

目标：将 `common_preflight` 稳定化后提取为核心 workspace crate。

- Phase 1: `odmr-smb100a` 扩展共享传输层
- Phase 2: M3 工具逐个切换到共享 crate
- Phase 3: `odmr-oe1022d` / `odmr-maynuo-m8812` 共享层审计

## 安全约束

- 所有 lab 工具均为只读或 human-in-the-loop 模式（ADR-004）
- AI 禁止直接控制硬件输出
- SCPI 命令只通过硬编码 allowlist，默认拒绝一切未列入的命令
- 所有提交前需通过 `scripts/check-consistency.sh` 和 `cargo clippy --workspace`

# Architecture

> 来源：docs/prd/01_architecture_prd_v0.2.md
> 本文件是 AGENTS.md 引用的权威架构入口，定义 crate 分层、依赖方向、模块边界。

## 分层模型（6 层 + Python 离线层）

```
Layer 6: Python Offline   python/analysis/  python/recipe_tools/
Layer 5: GUI              apps/desktop/     (Tauri/Web)
Layer 4: Application API  Tauri commands    (apps/desktop/src-tauri/)
Layer 3: Runtime          odmr-executor  odmr-logging  odmr-replay  odmr-harness  odmr-live-server
Layer 2: Domain           odmr-recipe  odmr-compiler  odmr-safety  odmr-config  odmr-preflight
Layer 1: Drivers          odmr-smb100a  odmr-oe1022d  odmr-maynuo-m8812  odmr-mag  odmr-laser  odmr-device
Layer 0: Types            odmr-types
```

## Crate 职责矩阵

### Layer 0 — 类型基座

**`odmr-types`** — 全仓库共享的基础类型，零依赖。
- 拥有：DeviceId, RecipeStep, Event, RunId, Timestamp, 错误类型枚举
- 不拥有：任何 I/O、任何设备协议、任何序列化格式选择
- 被：所有 crate 依赖
- 依赖：无（std only）

### Layer 1 — 设备驱动

**`odmr-device`** — 设备抽象 trait 和 DeviceManager。
- 拥有：Device trait, DeviceManager, ResourceLease, ConnectionState
- 不拥有：具体设备的 SCPI 指令、串口协议、硬件型号常量

**`odmr-smb100a`** — SMB100A RF 信号源 SCPI 驱动。
- 拥有：SMB100A 全部 SCPI 指令封装、频率/功率设置、扫描模式、内部 RF sweep 命令集
- 依赖：odmr-device, odmr-types
- 被：odmr-executor（通过 odmr-device trait）

**`odmr-oe1022d`** — OE1022D 锁相放大器 USB 驱动。
- 拥有：OE1022D 串口协议、RALL? 采集帧解析、锁相参数设置、RallCollector（Producer-Consumer 连续采集）
- 依赖：odmr-device, odmr-types
- 被：odmr-executor（高频采集链路）、odmr-live-server（实时 sidecar）、odmr-replay（帧解析）

**`odmr-maynuo-m8812`** — Maynuo M8812 可编程直流电源串口驱动。
- 拥有：USB-to-serial 传输、SCPI-like 指令封装、电流/电压/输出控制
- 依赖：odmr-device, odmr-types
- 被：odmr-mag（磁场控制规划层）

**`odmr-mag`** — 三轴磁场控制规划与协议边界模块。
- 拥有：Maynuo M8812 协议数据模型、线圈矩阵/电流换算、命令计划、零点锁定工作流、顺序轴运行、磁安全策略、运行时桥接类型（MagRuntimeCommand / MagReadback / MagCleanupReport）
- 依赖：odmr-device, odmr-types, odmr-maynuo-m8812
- 当前状态：M5A 真实硬件已验证

**`odmr-laser`** — CNI 激光器 PSU-SR 二进制串口驱动。
- 拥有：二进制协议帧封装（`0x55 0xAA` header）、`LaserClient`、`LaserFrame`、功率设置/开关控制/echo 检查
- 依赖：odmr-types, serialport
- 当前状态：活跃

### Layer 2 — 领域逻辑

**`odmr-config`** — Canonical JSON 配置入口。
- 拥有：`StationConfig`、`DeviceDefaults`、安全策略、清理策略、ArtifactPolicy、ReplayDefaults、FeatureFlags；legacy profile 兼容加载；手册默认值固化
- 依赖：odmr-types, serde, serde_json
- 状态：活跃（已从占位转正）

**`odmr-recipe`** — Recipe 数据结构与 JSON Schema 验证。
- 拥有：Recipe 反序列化、schema 验证、recipe 遍历/展开、SystemScanRecipe
- 依赖：odmr-types
- 不拥有：编译逻辑（那是 odmr-compiler 的事）

**`odmr-compiler`** — Recipe → resolved_recipe 编译管线。
- 拥有：参数展开、step 依赖解析、timing 计算、dry-run 生成、system scan 展开
- 输入：recipe.json
- 输出：resolved_recipe.json, safety_report.json, dry_run_plan.json
- 依赖：odmr-recipe, odmr-safety, odmr-types

**`odmr-safety`** — 安全互锁策略。
- 拥有：SafetyPolicy trait, InterlockEngine, 参数边界检查, 急停逻辑、system scan 安全报告
- 依赖：odmr-types
- 不拥有：GUI 层安全展示、硬件急停按钮驱动（那是 odmr-device + executor 的事）

**`odmr-preflight`** — 统一站级设备预检。
- 拥有：设备发现、身份验证、安全状态探测、跨进程设备锁（POSIX flock）、StationLedger 持久化、预检报告生成
- 依赖：odmr-types, serialport, fs2, odmr-config
- 位置：Layer 1+2 边界 — 跨越驱动探测（Layer 1）和领域级预检编排（Layer 2）

### Layer 3 — 运行时服务

**`odmr-executor`** — Recipe 执行引擎。
- 拥有：执行状态机、step 调度、设备命令编排、实时采集协调；mock-run + hardware-run 双模式
- 依赖：odmr-device, odmr-recipe, odmr-safety, odmr-logging, odmr-config
- 不拥有：GUI 控制逻辑

**`odmr-logging`** — 数据落盘。
- 拥有：RawRecorder（raw bin 写入）、IndexWriter（index.jsonl）、EventWriter
- 依赖：odmr-types
- 不拥有：数据格式转换（CSV/Parquet 导出由 Python 离线层负责）

**`odmr-replay`** — 离线回放。
- 拥有：Canonical `.rall` 运行产物回放、ReplaySession、ReplayMode（OriginalTimestampPaced / ParseOnly / AsFastAsPossible）、legacy rawbin → canonical 迁移
- 依赖：odmr-oe1022d, odmr-types, serde, serde_json
- 状态：活跃（已从占位转正）

**`odmr-harness`** — Mock/测试 harness。
- 拥有：FakeDevice 实现、mock 设备注册、测试 fixture 工具
- 依赖：odmr-device, odmr-types
- 被：所有集成测试

**`odmr-live-server`** — 实时 trace HTTP sidecar。
- 拥有：OE1022D 帧消费、ring buffer、Actix-web HTTP 服务（`/api/trace`, `/api/stats`）
- 依赖：odmr-oe1022d, actix-web
- 状态：Phase 1 demo，非生产实时链路的一部分

### Layer 4–5

**`apps/desktop/`** — Tauri 桌面应用。包含 Tauri Command API（Layer 4）和 Web 前端（Layer 5）。

### Layer 6 — Python 离线

**`python/recipe_tools/`** — Recipe 生成、检查、批量工具。
**`python/analysis/`** — 离线数据分析：谱线拟合、磁场反演、ML 训练。

## 依赖方向规则

```
规则 1: 禁止反向依赖
  Layer N 不能依赖 Layer N+1 及以上的任何模块。
  例: odmr-device 不能 import odmr-executor。

规则 2: 禁止循环依赖
  同一 Layer 内的 crate 之间尽量不互相依赖。
  例: odmr-oe1022d 和 odmr-smb100a 不互相引用。

规则 3: 跨层只能通过 trait 接口
  例: executor 通过 Device trait 调用驱动，不直接 import odmr-smb100a。

规则 4: GUI/Python 禁止直接访问硬件
  GUI 只能调 Tauri Command API。
  Python 只能读已落盘的数据文件。
```

## 关键数据流

```
实时采集链路（热路径）:
  OE1022D → odmr-oe1022d → odmr-executor → odmr-logging (raw bin + index)
  OE1022D → odmr-oe1022d → odmr-live-server (HTTP sidecar 实时图表)

Recipe 执行链路:
  recipe.json → odmr-recipe (validate) → odmr-compiler (resolve)
  → odmr-safety (check) → odmr-executor (run) → odmr-logging (record)

回放链路:
  raw/*.rall + index.jsonl → odmr-replay (canonical playback)
  legacy rawbin → odmr-replay (migrate → canonical → playback)

离线分析链路:
  raw bin + index.jsonl → python/analysis/ → parquet → ML/plot
```

## 参考

- 各 PRD 详情: `docs/prd/0*_prd_v0.2.md`
- 技术选型原因: `docs/adr/ADR-*.md`
- 进行中的设计决策: `docs/decisions/`

## 实现备注

- `tools/lab/` 下的 M3 SMB100A 工具已经从单文件原型向模块化实现迁移。
- 其中 `smb100a_fm_mod_microtest` 曾出现约 3000 行 `main.rs` 的阶段性实现，随后拆分为独立模块，保留 `main.rs` 作为薄入口。这一经验可视为后续 lab bring-up 工具的默认实现约束。
- `odmr-preflight` 是从 `tools/lab/common_preflight` 提取出来的 workspace crate，遵循同样的模块化原则。

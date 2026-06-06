# AGENTS.md

> 本文件面向 AI 编码智能体。阅读者被假设为对项目零先验知识。
> 项目主要使用中文撰写文档与注释，代码风格遵循 Rust / TypeScript 社区惯例。

## 项目概述

**odmrctl-web** 是 ODMR（Optically Detected Magnetic Resonance）自动化采集平台，面向 NV 色心 ODMR 实验。核心能力包括：设备编排、recipe 执行、高频采集、数据落盘与离线分析。

当前阶段：**GUI-M0 Mock Viewer 已完成；M4.1 recipe dry-run viewer 已完成；M5A RF + Mag + OE 最小组合实验已真实硬件验证；P6 common_preflight / StationLedger / DeviceLock 已激活；连接层进入 P6 固化阶段。**

- **Rust 后端**：设备编排、recipe 编译与安全检查、执行引擎、高频采集、raw-first 数据落盘。
- **前端**：Tauri v2 + React + Vite 桌面应用，仅做交互展示，**禁止任何硬件访问**。
- **Python**：仅离线分析与 recipe 生成工具，**不参与实时链路**。当前目录下仅有 `.gitkeep`（预留）。

## 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 后端核心 | Rust (Edition 2021) | 14 crate 的 Cargo workspace |
| 桌面 GUI | Tauri v2 | 跨平台 Webview 桌面壳 |
| 前端框架 | React 18 + TypeScript 5 | Vite 5 构建，React Router v6 路由 |
| 包管理 | pnpm | `apps/desktop/` 内使用；`.npmrc` 启用 `shamefully-hoist=true`、`strict-peer-dependencies=false`、`auto-install-peers=true` |
| Python 离线 | （预留） | `python/analysis/`、`python/recipe_tools/` 当前仅含 `.gitkeep` |
| CI/CD | GitHub Actions | `.github/workflows/ci.yml` |

## 仓库导航

| 路径 | 内容 | 何时查阅 |
|------|------|----------|
| `docs/prd/` | 13 份产品需求文档（`00`~`12`，总纲 + 子模块） | 理解功能边界与业务意图 |
| `docs/adr/` | 8 份架构决策记录（`ADR-001`~`ADR-008`） | 理解技术选型的原因 |
| `docs/architecture/ARCHITECTURE.md` | 分层模型、crate 职责、依赖方向 | 写代码前的入口 |
| `docs/decisions/` | 进行中的设计决策 | 开发中遇到选择时写这里 |
| `crates/` | Rust workspace 各 crate | 每个 crate 的 `README.md` 定义其边界 |
| `apps/desktop/` | Tauri 桌面应用（GUI-M0） | GUI 入口；当前为 mock-only |
| `python/` | recipe 工具 + 离线分析（预留） | 不参与实时链路 |
| `schemas/` | 9 份 JSON Schema（recipe、resolved_recipe、safety 等） | 验证 recipe 输入 |
| `examples/` | 示例 recipe 与设备指令 JSON | 参考用法 |
| `tests/` | 集成测试 fixture + golden 数据（预留） | mock-first 开发 |
| `tools/` | Lab bring-up 工具（M2–M3 阶段） | 硬件发现、只读快照、受控微测试、双设备 sweep / recipe-shaped run |
| `reverse_application/` | 旧系统逆向工程产物（C# 反编译、日志、协议分析） | 参考遗留系统行为 |

## 架构硬约束

这些是不可协商的规则，在 `docs/architecture/ARCHITECTURE.md` 中详细定义：

1. **依赖方向固定**：Layer 0 (类型) → Layer 1 (驱动) → Layer 2 (领域) → Layer 3 (运行时) → Layer 4 (API) → Layer 5 (GUI)。禁止反向依赖。
2. **单一硬件入口**：所有硬件访问必须经过 `DeviceManager` → `ResourceLease` → `Driver`。GUI、AI、Python 禁止直接访问硬件。
3. **实时链路隔离**：采集线程只做 read/timestamp/parse/buffer/write。CSV 导出、拟合、分析不能进入实时链路。
4. **Recipe 驱动**：实验执行必须来自 `recipe.json` → compiler → safety → dry-run → 人工批准 → executor。AI 不能直接发送硬件命令。
5. **Raw-first 数据**：实时阶段只写 raw bin + `index.jsonl` + `events.jsonl`。实验后再生成 parquet/csv。
6. **GUI-M0 硬件隔离**：`apps/desktop/src-tauri/Cargo.toml` 明确禁止依赖任何硬件 crate（`odmr-executor`、`odmr-smb100a`、`odmr-oe1022d`、`odmr-device`、`odmr-compiler`、`odmr-safety`、`odmr-logging`）。

### 分层模型（6 层 + Python 离线层）

```
Layer 6: Python Offline   python/analysis/  python/recipe_tools/
Layer 5: GUI              apps/desktop/     (Tauri/Web)
Layer 4: Application API  Tauri commands    (apps/desktop/src-tauri/)
Layer 3: Runtime          odmr-executor  odmr-logging  odmr-replay  odmr-harness
Layer 2: Domain           odmr-recipe  odmr-compiler  odmr-safety  odmr-config
Layer 1: Drivers          odmr-smb100a  odmr-oe1022d  odmr-maynuo-m8812  odmr-device  odmr-mag
Layer 0: Types            odmr-types
```

### Crate 职责速查

| Crate | Layer | 核心职责 | 关键依赖 | 状态 |
|-------|-------|----------|----------|------|
| `odmr-types` | 0 | `DeviceId`、`RecipeStep`、`Event`、`RunId`、`Timestamp`、错误枚举 | 无（std only） | 活跃 |
| `odmr-device` | 1 | `Device` trait、`DeviceManager`、`ResourceLease`、`ConnectionState` | `odmr-types` | 活跃 |
| `odmr-smb100a` | 1 | SMB100A SCPI 指令封装、频率/功率/扫描、LAN socket `Device` 实现 | `odmr-device`, `odmr-types` | 活跃 |
| `odmr-oe1022d` | 1 | OE1022D 串口协议、`RALL?` 帧解析、锁相参数、`Device` 实现 | `odmr-device`, `odmr-types` | 活跃 |
> **OE1022D 硬件实测要点**（固件 V6.3211110, SN:D6130220）：
> - **RALL?** 是唯一数据读取路径。12288 字节 f64 BE 二进制帧（20参数×50点），帧内1ms间距硬件保证。
> - **性能**：单帧读取 12.0ms（机械上限 83.7fps），设备刷新 ~48ms，有效去重帧率 20.8fps / ~1040 pts/sec。
> - **Pipeline 不支持**：连发多个 RALL? 只应答第一个，后续返回垃圾。
> - **读取规范**：节拍 48ms，每帧前 `clear(Input)`，去重用 X[0] 值比较。禁止固定 sleep >5ms。
> - **TRCAD? / TRCA? / TRCB? 不存在**。Buffer 子系统（SRATD/SLEND/SSLED 等）只能配置，不能读数据。
> - **稳定性与数据保留**：30 分钟连续真机验证通过（35,713 帧，1.2% dup，13.3ms 读取无漂移，零过载零解析错误）。采集阶段保留 12288B 原始二进制（.rall）不提取摘要 — 离线再按需解析任意参数/采样点。这是 ADR-005（raw-first）在 OE1022D 的落地。
> - **RallCollector**（`collector.rs`）：Producer-Consumer 模式，独立轮询生产线程 + bounded mpsc channel(8)。48ms 节拍、X[0] 去重、fast-poll 1ms 重试。Drop 非阻塞（不 join producer）。
> - 详见 `docs/decisions/oe1022d-rall-continuous-benchmark.md`、`docs/decisions/oe1022d-rall-stability-validation.md`
| `odmr-maynuo-m8812` | 1 | Maynuo M8812 串口 SCPI 指令封装、电流/电压/输出控制、`Device` 实现 | `odmr-device`, `odmr-types` | 活跃 |
| `odmr-mag` | 1 | 三轴磁场控制数据模型、Maynuo M8812 协议规划层、零点锁定/命令计划 | `odmr-device`, `odmr-types` | 活跃（M5A 真实硬件已验证） |
| `odmr-config` | 2 | 配置文件解析、设备地址登记、采集参数默认值（预留） | `odmr-types` | **占位**（仅 `placeholder()`） |
| `odmr-recipe` | 2 | Recipe JSON 反序列化、Schema 验证、遍历/展开、SHA-256 哈希 | `odmr-types`, `serde`, `serde_json`, `sha2`, `hex` | 活跃 |
| `odmr-compiler` | 2 | Recipe → `resolved_recipe` + `dry_run_plan.json`；参数展开、拓扑排序、timing | `odmr-recipe`, `odmr-types`, `serde`, `serde_json` | 活跃 |
| `odmr-safety` | 2 | `SafetyPolicy` trait、`InterlockEngine`、参数边界检查、急停逻辑、安全报告 | `odmr-recipe`, `odmr-types`, `serde`, `serde_json` | 活跃 |
| `odmr-executor` | 3 | 执行状态机、step 调度、设备命令编排、实时采集协调 | `odmr-recipe`, `odmr-compiler`, `odmr-safety`, `odmr-logging`, `odmr-device`, `odmr-smb100a`, `odmr-oe1022d`, `odmr-types` | 活跃 |
| `odmr-logging` | 3 | `RawRecorder`（raw bin）、`IndexWriter`（`index.jsonl`）、`EventWriter`（`events.jsonl`） | `odmr-types`, `serde`, `serde_json` | 活跃 |
| `odmr-replay` | 3 | 从 raw bin + index 重建采集数据流（预留） | `odmr-logging`, `odmr-types` | **占位**（仅 `placeholder()`） |
| `odmr-harness` | 3 | `FakeDevice` 实现、mock 设备注册、测试 fixture 工具 | `odmr-device`, `odmr-types` | 活跃 |

> **占位 crate 说明**：`odmr-config` 与 `odmr-replay` 当前为预留占位，源码仅含 `pub fn placeholder() {}`，`Cargo.toml` 无 `description` 字段。后续迭代中实现。

## 构建与运行

### Rust Workspace

```bash
# 格式化（必须无差异）
cargo fmt --all -- --check

# Clippy（警告视为错误）
cargo clippy --workspace --all-targets -- -D warnings

# 运行全部测试（含 crate 内单元测试与集成测试）
cargo test --workspace
```

### GUI-M0（Mock Viewer）

```bash
cd apps/desktop
pnpm install
pnpm tauri dev        # 开发模式，热重载，端口 1420
# 或
pnpm tauri build      # 发布构建
```

> GUI-M0 当前为 **mock-only**：所有数据来自构建时 bundl 的静态 snapshot，运行时无 fs/fetch；Start Run / Pause / Stop / Emergency Stop 等按钮全部禁用；无 serial / USB / VISA / TCP / SCPI 代码。

### Tauri 配置要点

- `tauri.conf.json` 中 `identifier` 为 `com.odmrctl.gui.m0`
- 开发服务器固定端口 `1420`（`strictPort: true`）
- 前端构建输出目录为 `../dist`
- `bundle.targets = "all"`，输出平台原生安装包

### 近期里程碑状态

- **GUI-M0**：已完成并保持 mock-only 边界。
- **M2**：已完成硬件发现、只读快照、OE1022D 实采、RALL 捕获、桥接与 shadow run。
- **M3**：已进入 SMB100A 受控 RF / FM/MOD 微测试、双设备软件步进 sweep、extended sweep、recipe-shaped run。
- **M5A**：RF + Mag + OE 最小组合实验已完成真实硬件验证；common_preflight 已统一设备预检；P6 连接层进入固化阶段。
- **M5B**：未开始。目标为多磁轴 ODMR 采集。
- **M4.2**：未开始。目标为 GUI run launcher（recipe 驱动的运行启动器）。

### GUI 当前状态

| 功能 | 状态 |
|------|------|
| M4.1 recipe dry-run viewer | ✅ 已完成 |
| M5A artifact viewer 类型 + Tauri 命令 | ✅ 已完成（read-only） |
| M5A artifact viewer 页面/路由 | ⏳ 未实现 |
| GUI run launcher | ⏳ 未实现 |
| 硬件控制按钮 | ❌ 不存在（架构约束） |

**架构约束：** GUI 禁止直接调用 lab 工具作为 shell 子进程进行真实运行。未来的 GUI run launcher（M4.2）必须通过类型化的 Rust 运行时 API（Layer 3/4 executor）发起实验，而非直接调用 `common_preflight` 或 lab 工具。

### Lab 工具分类

| 分类 | 工具 | 说明 |
|------|------|------|
| 产品运行时候选 | `common_preflight`（稳定化后）、`rf_mag_oe_minimal_run` 概念 | 未来提取为 workspace crate |
| 实验室联调专用 | `maynuo_m8812_identity_probe`、`zero_baseline`、`recur_microtest`、`sequential_axis_run`、`smb100a_*`、`oe1022d_*` 等 | M2–M4 单设备/单阶段工具 |
| OE1022D 稳定性验证 | `oe1022d_buffer_probe --stability-test` | 30min 真机连续采集，输出 .rall 原始二进制 + .csv 元数据，已验证无泄漏 |
| 诊断专用 | `visa_probe` | VISA A/B 基准测试 |
| GUI 只读支持 | M5A artifact viewer 类型与命令 | 解析产物文件，无硬件访问 |
| 废弃/遗留 | 无 | — |

## 测试策略

1. **Mock-first 开发**：新功能先在 `odmr-harness` 中用 `FakeDevice` 验证，再对接真实硬件。
2. **单元测试**：分散在各 crate 的 `src/` 内，随 `cargo test --workspace` 运行。
3. **集成测试**：放在各 crate 的 `tests/` 目录下，如：
   - `crates/odmr-recipe/tests/recipe_integration_tests.rs` — 验证 `examples/` 下所有 JSON 可被解析
   - `crates/odmr-compiler/tests/generate_examples.rs`
   - `crates/odmr-executor/tests/run_mock_end_to_end.rs`
   - `crates/odmr-logging/tests/generate_run_directory.rs`
   - `crates/odmr-safety/tests/generate_safety_reports.rs`
4. **Fixture / Golden**：`tests/fixtures/` 与 `tests/golden/` 预留，用于跨 crate 的集成测试数据。
   - 当前 fixture 示例：`tests/fixtures/oe1022d_rall/rall_frame_*.raw` + `rall_capture_index.jsonl`

### Coding Agent 设备连接/测试提示（必读）

> 本条专门针对 AI coding agent。

#### 规则 1：区分"纯软件工作"和"需要硬件的工作"

**以下工作不需要真实硬件，用 mock/harness 完成即为标准流程：**

| 工作类型 | 示例 | 测试方式 |
|----------|------|----------|
| Recipe parser / validator | `odmr-recipe` 新增 recipe kind | `cargo test -p odmr-recipe`，用 `include_str!` 加载 example JSON |
| Recipe compiler / expander | `odmr-compiler` 新增展开逻辑 | 纯单元测试，无需任何 transport |
| Safety 静态检查 | `odmr-safety` 新增检查项 | 纯逻辑判断，无需设备 |
| GUI 类型/预览适配 | `apps/desktop` 新增 recipe kind 识别 | `pnpm tsc --noEmit` |
| Schema / example JSON | 新增 `examples/*.json` | 集成测试验证解析+序列化 |

**以下工作才需要真实硬件，且必须有人工操作员在场：**

| 工作类型 | 示例 | 前提条件 |
|----------|------|----------|
| Lab bring-up 工具端到端验证 | `rf_mag_oe_minimal_run` 真实跑一次 | 设备上电、操作员在场、实验室安全确认 |
| Driver 真实 transport 验证 | `odmr-smb100a` LAN socket 通信 | 设备联网、VISA 可用 |
| 采集链路真实数据验证 | OE1022D `RALL?` 实采 | 设备串口连接、示波器确认 |
| Preflight 真实设备发现 | `common_preflight` 枚举 station | 设备物理连接 |

**核心原则：**
- **Mock-first 是设计意图，不是临时 workaround。** `odmr-harness` 中的 `FakeDevice` 实现与真实设备驱动共享同一 `Device` trait，通过 harness 测试即视为有效验证。
- **Layer 2（domain）和 Layer 3（runtime）的新功能，不应以"硬件 unavailable"为由 block。** 这些层只操作数据结构（`Recipe`、`ResolvedRecipe`、`SafetyReport`），不直接访问硬件。
- **只有 Layer 1（driver）和 `tools/lab/` 的端到端工具才需要真实硬件。** 且这些工具的测试也先在 `FakeTransport` / `FakeDevice` 上跑通，再预约 lab 时间做真实验证。

#### 规则 2：需要硬件时必须走标准化 preflight，禁止自己瞎连接

如果你的任务涉及真实设备（SMB100A、OE1022D、Maynuo、CNI Laser），**必须按以下顺序执行**，禁止跳过步骤或自行发明连接逻辑：

**Step 1 — 读现有流程文档和审计（必读，不要猜）**
- `docs/lab-bringup/preflight_checklist_template.md` — **核心执行模板**：Phase A (passive preflight) → Phase B (operator approval) → Phase C (armed execution) → emergency procedures → post-run verification
- `docs/lab-bringup/device_connection_initialization_audit.md` — 设备连接审计根因、状态机、cleanup 标准
- `docs/lab-bringup/p6_m5a_audit_b_device_connection_contract.md` — 各设备 probe 分类、auto-discovery 风险
- `docs/lab-bringup/cni_laser_preflight_plan.md` — 激光器 off-only 安全边界
- `crates/odmr-preflight/README.md` — 已提取的共享 preflight crate API

**Step 2 — 使用 common_preflight 做统一设备预检**
```bash
cd tools/lab/common_preflight
cargo run -- --station-profile <path> --preflight-only
```
或调用 `odmr-preflight` crate 的 API：
```rust
use odmr_preflight::{run_station_preflight, StationProfile};
let profile = StationProfile::load("station.json")?;
let report = run_station_preflight(&profile, Some(&ledger_path), true)?;
```

**Step 3 — 检查 preflight report**
Preflight 会生成 `station_preflight_report.json`，必须确认：
- `all_devices_reachable: true`
- `all_identities_verified: true`
- `all_safe_states_confirmed: true`
- 各设备的 `error_queue` 为空
- SMB100A: `rf_output: off`, `modulation: off`, `fm: off`
- Maynuo: `output_on: false`, `current_ma` 在 tolerance 内

**Step 4 — 设备锁持有期间才能执行真实 run**
```rust
let (report, locks) = run_station_preflight_with_locks(&profile, ...)?;
// locks 持有期间，其他进程无法占用同一设备
// run 结束后 locks drop，设备释放
```

**Step 5 — cleanup 后必须验证**
Maynuo cleanup 标准顺序：
```text
CURR 0 → OUTP 0 → wait 500ms → MEAS:CURR? → verify → SYST:LOC
```
SMB cleanup：必须发送 `OUTP OFF`，无论正常完成还是异常退出。

**禁止行为（已造成过问题）：**
- ❌ 自己写 TCP socket 直接连 SMB100A 而不走 preflight
- ❌ 自己枚举 `/dev/cu.*` 猜测哪个是 OE1022D / Maynuo
- ❌ 跳过 `*IDN?` 身份确认直接发配置命令
- ❌ 在 preflight 失败时继续执行实验步骤
- ❌ Maynuo cleanup 后直接 `SYST:LOC` 不等待电流衰减
- ❌ 用 `*RST` 重置 OE1022D（会清空数据 buffer）
- ❌ 激光器不经过 off-only preflight 直接并入 RF/Mag/OE run

## 代码风格指南

### Rust

- **Edition 2021**，所有 crate 统一使用。
- **License header**：`license = "MIT OR Apache-2.0"` 写在每个 crate 的 `Cargo.toml` 中。
- **文档**：每个 crate 根 `lib.rs` 必须有 crate-level doc comment（`//!`），说明层级、职责、依赖、不负责什么。
- **模块文档**：公开类型与 trait 必须带 doc comment（`///`）。
- **格式化**：`cargo fmt --all -- --check` 零差异。
- **静态检查**：`cargo clippy --workspace --all-targets -- -D warnings` 零警告。
- **错误类型**：各 crate 定义自己的 `Error` 枚举（如 `RecipeError`、`CompileError`、`ExecutorError`），避免滥用 `String`。
- **命名**：
  - 类型/枚举：`PascalCase`
  - 函数/变量：`snake_case`
  - 常量：`SCREAMING_SNAKE_CASE`
  - trait 名：名词或 `CanXxx` / `IsXxx` 形式

### TypeScript / React（前端）

- **严格模式**：`tsconfig.json` 中 `"strict": true`，并启用 `noUnusedLocals`、`noUnusedParameters`、`noFallthroughCasesInSwitch`。
- **路径别名**：`@/` 映射到 `src/`，在 `vite.config.ts` 与 `tsconfig.json` 中同步配置。
- **CSS**：使用 CSS 变量（`styles/tokens.css`）管理设计 token，`app.css` 管理组件样式。
- **Mock 数据**：所有 mock 数据放在 `src/mock-data/` 下，并配有 `MockModeContext` 统一管理 mock 状态。

## 安全与合规

### 禁止项（由自动化脚本守卫）

| 禁止行为 | 原因 | 守卫脚本 |
|----------|------|----------|
| 前端直接访问硬件（serial/USB/VISA/SCPI socket/TCP） | 架构约束：前端只能调 Tauri Command API | `check-frontend-hardware.sh` |
| 实时 crate 中出现 CSV writer | ADR-005：实时阶段只允许 raw bin + jsonl | `check-realtime-csv.sh` |
| AI 直接控制活硬件 | ADR-004：AI 只能操作 recipe/数据，不能绕过 safety 直接发硬件命令 | 架构审查 + PR 审核 |
| 硬件 crate 被 GUI-M0 引入 | GUI-M0 为 mock-only，禁止引入真实驱动 | `apps/desktop/src-tauri/Cargo.toml` 显式注释禁止 |

### 关键数据流

```
实时采集链路（热路径）:
  OE1022D → odmr-oe1022d → odmr-executor → odmr-logging (raw bin + index)

Recipe 执行链路:
  recipe.json → odmr-recipe (validate) → odmr-compiler (resolve)
  → odmr-safety (check) → odmr-executor (run) → odmr-logging (record)

离线分析链路:
  raw bin + index.jsonl → python/analysis/ → parquet → ML/plot
```

## 机械化检查（CI / Pre-commit）

仓库根目录执行：

```bash
bash scripts/check-consistency.sh      # C1~C5：crate 目录、PRD/ADR 编号、README 一致性
bash scripts/check-docs-links.sh       # docs/ 内 Markdown 内部链接有效性
bash scripts/check-prd-adr-index.sh    # PRD/ADR 文件存在性与交叉引用
bash scripts/check-frontend-hardware.sh # 前端禁止硬件访问模式
bash scripts/check-realtime-csv.sh     # 实时 crate 禁止 CSV writer
bash scripts/check-agents-md.sh        # 关键目录 AGENTS.md 存在性
bash scripts/check-schema-examples.sh  # examples/ JSON 符合 schema 且被测试覆盖
bash scripts/check-command-catalog.sh  # 设备指令目录编译、测试、与 JSON 源一致
```

启用 pre-commit hook（已配置）：

```bash
git config core.hooksPath .githooks
```

`.githooks/pre-commit` 会根据暂存文件类型自动触发对应子集检查：
- `Cargo.toml` / `crates/` / `README` / `AGENTS` / `docs/architecture/` 变更 → `check-consistency.sh`
- `examples/` / `schemas/` / `crates/odmr-recipe/` 变更 → `check-schema-examples.sh`
- 设备 crate / 指令 JSON 变更 → `check-command-catalog.sh`
- `docs/` 变更 → `check-docs-links.sh`
- `apps/desktop/` 变更 → `check-frontend-hardware.sh`
- 实时 crate 变更 → `check-realtime-csv.sh`

### CI 流水线

`.github/workflows/ci.yml` 包含两个 job：

1. **Mechanical Checks**（`ubuntu-latest`）：运行 `check-docs-links.sh`、`check-prd-adr-index.sh`、`check-frontend-hardware.sh`、`check-realtime-csv.sh`、`check-agents-md.sh`、`check-consistency.sh`。
2. **Rust Build & Test**（`ubuntu-latest`）：安装 stable Rust（含 `rustfmt`、`clippy`）→ `cargo fmt --check` → `cargo clippy -D warnings` → `cargo test --workspace` → `check-schema-examples.sh` → `check-command-catalog.sh`。

触发条件：`push` 与 `pull_request` 到 `main` / `master`。

## 开发流程

1. **新功能先写设计决策**：在 `docs/decisions/` 中记录选择依据。
2. **对照架构文档确认边界**：`docs/architecture/ARCHITECTURE.md` 与对应 crate 的 `README.md`。
3. **PRD-first**：每个实现任务必须引用 PRD 章节编号。
4. **mock-first**：设备交互先在 `odmr-harness` 中用 `FakeDevice` 验证。
5. **集成测试 fixture**：放 `tests/fixtures/`，golden 数据放 `tests/golden/`。
6. **recipe 示例**：放 `examples/`，schema 变更同步更新 `schemas/`。
7. **提交前**：运行 `bash scripts/check-consistency.sh` 与 `cargo test --workspace`。

## 最近实现经验

- `tools/lab/smb100a_fm_mod_microtest` 早期曾出现单文件 `main.rs` 约 3000 行的实现，后续已拆分为 `app`、`cli`、`types`、`timeline`、`artifacts`、`safety`、`transport`、`sequence`、`shutdown`、`tests` 等模块，`main.rs` 缩减为薄入口。
- 后续新增 lab 工具优先复用已模块化的 transport / safety / artifacts / timeline 结构，避免再次回到超长 `main.rs`。

## 部署说明

- **当前阶段**：GUI-M0 为 mock-only 桌面应用，无服务端部署需求。
- **构建产物**：`pnpm tauri build` 输出平台原生安装包（`.dmg` / `.app` / `.exe` / `.msi` / `.deb` / `.rpm` 等），由 Tauri v2 的 `tauri.conf.json` 中 `bundle.targets = "all"` 控制。
- **发布**：尚未配置自动 release 流水线；构建产物在本地 `apps/desktop/src-tauri/target/release/bundle/` 生成。

## 扩展阅读

- 各 PRD 详情：`docs/prd/0*_prd_v0.2.md`
- 技术选型原因：`docs/adr/ADR-*.md`
- 架构总图：`docs/architecture/ARCHITECTURE.md`
- 新人/智能体快速入口：本文件（`AGENTS.md`）

# 不接真机可完成任务决策清单

日期: 2026-06-09

目标: 把真实实验执行体系中不依赖真机的任务先收口到类型、配置、协议映射、回放、测试和文档。真机只用于最终 acceptance，不作为纯软件工作的阻塞条件。

依据手册:
- `docs/equipment_manual/smb100a/05_remote_control_basics.md`
- `docs/equipment_manual/smb100a/06l_source_subsystem.md`
- `docs/equipment_manual/oe1022d/02_fundamentals.md`
- `docs/equipment_manual/oe1022d/oe1022d_reference_signal_remote.md`
- `docs/equipment_manual/oe1022d/oe1022d_input_filter_remote.md`
- `docs/equipment_manual/oe1022d/05_oe1022d_rall_global_data_config_reading.md`
- `docs/equipment_manual/maynuo_dc-power-supply/m8812_remote_control_reference.md`
- `docs/equipment_manual/CNI Laser psu-sr/RS232语言协议_恒功率.md`

## 1. `odmr-config`

决策: `odmr-config` 是 canonical JSON 配置入口，负责 App/Station/DeviceDefaults 与手册默认值，不再把运行默认值散落在 GUI、profile 或 lab tool 内。

已完成:
- 定义 `AppConfig`、`StationConfig`、`DeviceDefaults`、`EffectiveRuntimeDefaults`。
- 支持 canonical station JSON 和 legacy station/profile JSON 兼容读取。
- 固化 SMB100A raw socket 5025、OE1022D RALL 12288B/50 points/50ms、Maynuo 9600 8N1 LF、CNI 9600 8N1。
- 新增 OE1022D PLL reference defaults 和 `PllReferenceContract`，可纯软件判断参考信号是否满足手册锁相阈值。

仍需真机验收:
- 用真实 station profile 加载后，对实际物理布线填入参考信号 contract，并确认 preflight/run 前能给出正确阻塞或放行。

## 2. SMB100A 与 OE1022D 锁相配置

决策: ODMR 实验使用 OE1022D Ch-B 外部参考作为锁相信号入口；SMB100A LF 输出只有在物理接到 OE reference input 且满足电平阈值时，才能作为锁相参考。

手册约束:
- OE 外部参考模式: `FMODD 2,0`。
- OE TTL 上升沿: `RSLPD 2,0`，TTL 高电平必须 > 3V，低电平必须 < 0.5V。
- OE 正弦过零: `RSLPD 2,1`，正弦参考必须 > 0.4Vpp。
- 参考频率低于 1Hz 时必须使用 TTL。
- 内部参考模式不使用 PLL，不能作为外部参考 PLL lock 目标。

当前理论判定:
- 如果当前配置是 SMB100A LF 方波 0.137V peak 接 OE TTL reference，则理论上不能 PLL lock，因为 0.137V 不满足 OE TTL 高电平 > 3V。
- 可行配置之一是独立 TTL/sync 参考输出接 OE reference，满足 high > 3V、low < 0.5V，并设置 OE `FMODD 2,0` + `RSLPD 2,0`。
- 可行配置之二是 SMB100A LF 正弦输出接 OE reference，幅度设置为 > 0.2V peak（即 > 0.4Vpp），并设置 OE `FMODD 2,0` + `RSLPD 2,1`。这要求物理接线确认为 LF analog output 到 OE reference input。

已完成:
- Tauri SMB panel 的“应用到设备”改为一次 typed command 下发频率、功率、LF 频率/电压/波形/阻抗、FM source/mode/deviation/state、MOD 总开关，并保持 RF OFF。
- Tauri OE panel 的 Ch-B 应用改为一次 typed command 下发输入源、接地、耦合、陷波、动态储备、灵敏度、时间常数、斜率、参考源、外部触发、相位、同步滤波。
- 修复 OE 陷波器索引: 0=Off, 1=50Hz, 2=100Hz, 3=Both。
- 默认 OE Ch-B 参考触发改为 TTL 上升沿，避免继续沿用旧截图的“过零检测”。

仍需真机验收:
- 确认实际参考线来自 TTL/sync 还是 SMB LF analog。
- 在 OE status 中验证 `PLL LOCKED` 从 0 变为 1，并记录参考频率读回。

## 3. `odmr-laser`

决策: 激光协议属于 Layer 1 driver，不允许 GUI panel 持有 CNI 二进制帧协议所有权。

已完成:
- 新增 crate 并实现 CNI serial config、协议帧、power/on/off/emergency off typed API。
- Tauri laser panel 通过 `odmr-laser` 调用，不再维护 fake 协议层。

仍需真机验收:
- 在人工安全确认下执行 off-only preflight、低功率 enable/disable、emergency off。

## 4. `odmr-replay`

决策: 当前 `.rall + index.jsonl + events.jsonl + metadata/summary` 是 canonical run 目录。旧 rawbin 只通过 adapter/migration 兼容。

已完成:
- 实现 canonical run manifest/metadata loader。
- 实现 `.rall` frame parser replay、step-scoped replay、parse-only replay。
- 实现 legacy rawbin adapter 和迁移入口。

仍需真机验收:
- 用真实最小 run 产物回放，核对 frame count、step alignment、trace continuity。

## 5. `odmr-mag` 与 `odmr-executor`

决策: 磁场运行期状态要有 typed runtime bridge；executor 保留 mock mode，同时提供 hardware mode 的软件编排接口。串口 I/O 仍在 `odmr-maynuo-m8812`，不回灌进 `odmr-mag`。

已完成:
- `odmr-mag` 增加 apply/readback/cleanup/zero-lock runtime bridge 类型与 fake transport 测试。
- `odmr-executor` 保留 mock regression，并提供 hardware execution config/report/control 类型与测试骨架。

仍需真机验收:
- Maynuo 三轴 apply/readback/cleanup 的真实 current decay 行为。
- executor hardware run 的完整 SMB + OE + Mag + Laser acceptance。

## 6. Tauri/GUI 边界

决策: GUI 只持有草稿和展示状态；所有真实下发走 typed Tauri command；前端不允许 SCPI/socket/serial。

已完成:
- SMB/OE panel 下发逻辑由散落多 command 拼接收口为设备级 typed apply command。
- OE status 增加 input/reserve/sensitivity/sync filter readback 字段。
- SMB status 增加 FM source/mode 与 LF impedance readback 字段。

仍需补齐:
- `experiment_plan` hardware run/stop/status 需要完全接入 executor runtime，而不是保留历史 placeholder/handoff 语义。
- artifact viewer 和 replay control API 需要统一读取 canonical run model。

## 7. 测试与检查

纯软件必须通过:
- `cargo test --workspace --no-fail-fast`
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --no-fail-fast`
- `pnpm tsc --noEmit` in `apps/desktop`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings`

真机 acceptance 不属于本清单完成条件，但必须在 release 前补:
- 最小真机 run。
- 中途 stop。
- 真实 run replay。
- 激光 emergency off。

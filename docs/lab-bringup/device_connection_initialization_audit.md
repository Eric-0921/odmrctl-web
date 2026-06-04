# Device Connection And Initialization Audit

日期：2026-06-04

## 结论

当前设备链路的主要问题不是某一个驱动或某一次连接超时，而是缺少统一的 station-level preflight。各 lab 工具都在局部实现 SMB100A、OE1022D、Maynuo M8812 的连接、身份确认、错误队列、safe-state reset 和最终清理，导致 agent 每次都要重新判断“设备是不是在线、是不是安全、是不是同一个设备、是不是能进入实验态”。

M3.5 已证明两设备 recipe-shaped run 的采集链路稳定。下一步不应该把更多真实设备直接塞进 run loop，而应该先把连接和初始化流程收敛成一套可复用的、可审计的、先被动后主动的流程。

推荐优先级：

1. 新增统一 `preflight-only` 流程，先验证所有设备身份和安全状态，再允许任何能量输出。
2. 把 SMB100A、OE1022D、Maynuo M8812、CNI Laser 的连接状态、身份、错误队列、safe reset、operator approval 写成统一 artifact。
3. 标准化 Maynuo cleanup 顺序，避免 local mode 后再 query，以及避免输出关闭后的电流衰减时间被误判。
4. 将激光器作为独立安全链路加入实验前置条件，先做 off-only / fake-first，不和 RF、磁场、OE 采集合并。
5. 后续再考虑把 lab 工具里的重复 transport/preflight 提取成共享模块。

## 手册约束摘要

### SMB100A

SMB100A 支持 LAN remote control。手册中明确存在 HiSLIP、VXI-11 和 raw socket 这几类 LAN 远控路径。

当前 lab 工具主要使用 raw TCP socket。这个方案能工作，也不要求安装 VISA，但它是最简网络通信路径，缺少更成熟 remote-control 栈里的 device clear、serial poll、SRQ、locking、remote/local 等能力。连接偶发失败时，raw socket 只能看到 TCP connect/read timeout，很难区分网络物理层、仪器 remote 状态、SCPI 队列状态或并发占用。

建议：

- 短期继续保留 raw socket，但加 fast TCP probe、错误分类、重连和统一 artifact。
- 中期安装 Rohde & Schwarz VISA 或 NI-VISA，并评估 HiSLIP/VXI-11 后端。
- 如果继续使用 raw socket，必须显式做进程级设备锁，避免多个 agent/tool 同时连同一台信号源。
- `SYST:ERR?` 会弹出 error queue 中最老的一条，preflight 应循环读取直到 `0`，并保留完整队列记录。

### OE1022D

OE1022D 手册有几个对当前流程很关键的点：

- 命令必须以 LF 或 CR 终止，否则不会执行。
- 不建议把多条命令用 `;` 塞成一行，因为执行阻塞且输入缓冲区只有 256 字符。
- `RALL?` 返回固定 12288 bytes。
- `RALL?` 资料写明是 USB2.0 指令，DB9 RS232 不支持；当前 `/dev/cu.usbmodem*` 这类 USB CDC 路径需要在文档里明确归类为 USB 链路，而不是普通 RS232。
- `RALL?` 数据中包含 overload 和 PLL lock 状态，应该从“只解析 B-X/B-Y 数值”升级到“同时记录质量状态”。
- IDN 字段固定长度，尾部出现 `\0` 是预期数据形态，不应该当成通信失败。应保留 raw identity，同时提供 cleaned display identity。

建议：

- 每次 `RALL?` 前清空 input buffer，避免上一次 incomplete frame 污染当前 frame。
- `RALL?` 必须按 fixed length 读取，少于 12288 bytes 应进入 quarantine / quality flag。
- preflight 增加 PLL lock 和 overload 状态检查，至少写入 artifact；是否 abort 由实验配置决定。
- 禁止在常规 preflight 里使用 `*RSTD`，因为它会 reset device 并清空 data buffers。

### Maynuo M8812

三轴磁场线圈目前没有正式说明书，只能从实测记录和工具行为建立 lab note。已有问题显示，cleanup 顺序和读回时机是核心风险：

- `CURR 0` 只是设定值，不代表实际输出电流已降到 0。
- `MEAS:CURR?` 读的是实际输出，输出关闭后需要等待数百毫秒。
- `SYST:LOC` 后再 query 不可靠，不能作为 cleanup 通过条件。
- 端口路径会变化，必须用 serial enumeration + `*IDN?` + SN 精确匹配绑定轴。

推荐 cleanup 标准：

```text
CURR 0
OUTP 0
wait 500 ms minimum
MEAS:CURR?
verify within tolerance
SYST:LOC
```

建议把这个顺序固化到所有 Maynuo 工具，并把 tolerance 设为基于实测噪声的电流阈值，而不是过紧的 mA 级瞬时判定。

### CNI Laser

激光器目前在 station schema 和 PRD 中出现，但还没有进入真实实验流程。手册显示这是比 RF 和磁场更需要先独立安全化的设备：

- RS232 参数为 9600, 8N1，无校验。
- 协议是二进制帧，不是 SCPI，不能用 `*IDN?` 识别。
- `Laser Off`、`Laser On`、`Set Power` 都是固定帧/校验和协议。
- 手册强调钥匙开关、interlock、急停、报警、指示灯和预热时间。
- 断开串口或重启后，上次功率设定可能仍然保留，不能假设 power setpoint 为 0。

建议激光器不要直接并入 M3/M4 run。应单独开 `Laser-M0` 到 `Laser-M3`：

1. `Laser-M0`：整理协议、人工安全 checklist、设备标签和 USB serial 绑定规则。
2. `Laser-M1`：fake driver + binary command builder + checksum tests，不接真实光。
3. `Laser-M2`：real off-only preflight，只允许识别串口和发送 Laser Off。
4. `Laser-M3`：低功率、人工批准、物理 shutter/interlock 确认后的 enable microtest。

在确认最大安全功率前，软件限值应采用保守值。现有资料里存在 0-150 与约 306 mW 的口径差异，不能由 agent 自行放宽。

## 现有代码风险

### SMB100A raw socket 仍可用，但不是最优长期方案

`recipe_two_device_run` 当前通过 TCP raw socket 连接 SMB100A。它已经有 allowlist、防 semicolon、query delay、set command 不读回等保护，适合 M3.5 的受控实验。

不足：

- connect 前没有 network/physical probe，ping 失败、端口不通、仪器睡眠和 SCPI timeout 会混在一起。
- 没有 VISA/HiSLIP/VXI-11 的 device clear、locking、serial poll。
- preflight 里有些状态被记录但没有形成统一 abort 策略。
- final safe-state 对 RF/MOD 查询较明确，但 FM 仍应增加 `FM:STAT?` 真实查询确认。

### 初始化顺序需要从“边连边配置”改为“两阶段”

建议所有真实 run 改成：

```text
Phase A: passive preflight
  connect all devices
  identify all devices
  check error queues / status / overload / lock
  issue safe reset where allowed
  write preflight artifacts

Phase B: armed execution
  require operator approval
  preload target state
  enable outputs only inside bounded step window
  immediately verify and record final safe state
```

现在一些工具会在所有设备都确认之前，对 SMB100A 做 FM/MOD preload。虽然还没有 RF output，但从第一性原理看，任何“会影响能量输出形态的状态”都应该晚于所有设备 passive preflight。

### Maynuo cleanup 需要统一修正

从现有工具看，不同 Maynuo 工具里的 cleanup 顺序不完全一致。只要存在 `CURR 0 -> OUTP 0 -> SYST:LOC` 后不等待、不测量的路径，就会复现已观察到的误判或残余电流读数问题。

应新增共享 helper，而不是在每个工具里复制 cleanup：

```text
maynuo_safe_zero_and_local(axis):
  SYST:REM
  CURR 0
  OUTP 0
  wait 500 ms
  MEAS:CURR?
  verify abs(current) <= tolerance
  SYST:LOC
```

### OE1022D frame 质量应该进入 artifact

M3.5 证明 330/330 frames parseable，但这不代表之后可以忽略 frame-level 状态。OE 手册给了 overload / PLL lock 字段，应该进入 `quality_flags` 或 step summary。否则 GUI 或 analysis 只能看到数值，看不到“数值是否可信”。

## 推荐统一 Preflight Artifact

每次真实工具运行前都应生成：

```text
preflight/
  station_preflight_report.json
  device_identity_snapshot.json
  device_connectivity_report.json
  safe_state_reset_report.json
  operator_approval.json
  manual_checklist.md
```

`station_preflight_report.json` 建议字段：

```json
{
  "schema_version": 1,
  "generated_at": "...",
  "station_profile": "...",
  "all_devices_reachable": true,
  "all_identities_verified": true,
  "all_safe_states_confirmed": true,
  "operator_approved": false,
  "devices": [
    {
      "device_id": "smb100a.main",
      "kind": "rf_source",
      "transport": "tcp_raw_socket",
      "address": "169.254.2.20:5025",
      "physical_probe": "tcp_port_open",
      "identity_raw": "...",
      "identity_display": "...",
      "error_queue": [],
      "safe_state": {
        "rf_output": "off",
        "modulation": "off",
        "fm": "off"
      },
      "warnings": []
    }
  ]
}
```

## 建议的连接状态机

统一所有真实设备工具的连接状态：

```text
Unseen
  -> PhysicalReachable
  -> TransportOpen
  -> IdentityVerified
  -> ErrorQueueKnown
  -> SafeStateConfirmed
  -> ArmedByOperator
  -> ActiveStep
  -> SafeShutdownConfirmed
```

禁止跳过 `IdentityVerified` 和 `SafeStateConfirmed` 直接进入 `ArmedByOperator`。如果任何设备失败，应进入 best-effort safe shutdown，并写失败 artifact，而不是让下一个 agent 从未知状态重新猜。

## Agent 经常卡在连接上的根因

1. 连接失败没有分层：物理网络、TCP 端口、串口枚举、协议响应、状态错误、身份不匹配被混在一个 timeout 里。
2. 没有 station fingerprint：每次都重新判断哪个 `/dev/cu.*` 是哪个设备。
3. 没有统一 safe-state ledger：异常退出后，下一次不知道设备上次是否完成 cleanup。
4. raw socket 缺少设备锁：多个进程可能同时访问 SMB100A。
5. Maynuo 和 Laser 不是 SCPI 标准仪器，不能套用 SMB100A 的连接模型。
6. OE1022D `RALL?` 是 fixed-size binary-like frame，不是普通 line query，读法和 timeout 必须独立建模。
7. 激光器尚未纳入 off-only preflight，未来一旦加入真实 run，会引入新的安全状态和初始化耗时。

## 实施路线

### P0: 文档和流程冻结

- 建立 `docs/lab-bringup/maynuo_m8812_lab_notes.md`，记录 SN 绑定、端口枚举、cleanup 顺序、读回 tolerance、已知失败签名。
- 建立 `docs/lab-bringup/cni_laser_preflight_plan.md`，先定义 off-only 安全边界。
- 为所有 real run 文档增加同一个 preflight checklist。

### P1: 轻量共享 preflight 工具

新增 `tools/lab/common_preflight/` 或等价共享模块，先不升格为核心 crate：

- `smb_probe`
- `oe_probe`
- `maynuo_probe`
- `laser_probe_off_only`
- `station_preflight_report` writer
- process lock / device lock helper

每个真实 lab tool 增加：

```text
--preflight-only
--require-preflight-report <path>
--station-profile <path>
```

### P2: 修正具体设备流程

- SMB100A：connect 前 TCP probe；preflight abort 策略；final `FM:STAT?`；完整 error queue drain artifact。
- OE1022D：identity raw/display 分离；input buffer clear；RALL status bits 进入 quality flags。
- Maynuo：统一 cleanup helper；wait + measure before local；基于 SN 的 axis binding 必须项。
- Laser：fake-first binary protocol；off-only real preflight；manual approval gate。

### P3: 后端架构收敛

当 lab preflight 稳定后，再考虑把公共能力上移：

- transport 和 identity 放入对应 driver crate。
- station profile 进入 `odmr-config`。
- run-time lease / state ledger 进入 `odmr-device` 或 executor 边界。

不要一开始就把未稳定的 lab 经验塞进核心 crate。先让实验室流程跑稳，再提升抽象层级。

## 当前建议的下一步

建议先做一个小而明确的 M3.6.x / M3.7 前置任务：

```text
M3.6.x Station Preflight Artifact

Input:
  station profile
  SMB100A address
  OE1022D port
  optional Maynuo SN map
  optional Laser serial hint

Output:
  preflight/station_preflight_report.json
  preflight/device_identity_snapshot.json
  preflight/safe_state_reset_report.json
  preflight/manual_checklist.md

Rules:
  no RF output
  no Maynuo output
  laser off-only only
  no experiment acquisition
  no GUI
```

这会直接减少 agent 在真实 run 前反复连接、试探和猜状态的时间，也会给后续 GUI read-only viewer 提供更可信的设备初始化证据链。

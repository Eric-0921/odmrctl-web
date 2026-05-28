# ADR-003: SMB100A 首版控制路径采用 SCPI over LAN Socket

## Status

Accepted

## Date

2026-05-28

## Context

ODMR 自动化系统需要控制 R&S SMB100A 微波信号源，用于设置 RF 频率、RF 功率、RF 输出状态、LF 输出、FM 调制、RF 频率扫频等参数。

系统总体架构已经确定：

```text
GUI 不直接访问硬件
AI 不直接访问硬件
Recipe / Executor 负责生成和执行设备命令
所有危险命令必须经过 schema 校验、safety 校验和 dry-run 展示
```

SMB100A 是 SCPI 仪器。根据 R&S SMB100A Operating Manual，SMB100A 支持 LAN 远程控制，并支持 VXI-11、HiSLIP 和 Socket communication。手册中还给出了 raw socket 资源字符串示例：

```text
TCPIP::192.1.2.3::5025::SOCKET
```

手册说明 socket communication 又称 Raw Ethernet communication，不一定要求远程控制端安装 VISA。SMB100A 通过 Telnet/socket 远程连接使用 5025 端口。

Rohde & Schwarz 官方同时提供 Python 的 RsInstrument 包。RsInstrument 是面向 R&S SCPI-based test and measurement instruments 的 Python 远程控制通信模块，适合作为验证路径和 fallback 路径。

但是，首版系统不应该把 SMB100A 控制绑定在 Python/RsInstrument 或 VISA FFI 上。原因是：

1. SMB100A 的核心控制命令是 SCPI 文本命令，首版只需要命令发送、查询、超时、错误检查和日志记录。
2. LAN socket 是最小依赖路径，不要求额外安装 VISA runtime。
3. Rust / Go 通过 TCP socket 发送 SCPI 在技术上可行。
4. 但 R&S 官方 first-class 示例主要覆盖 Python、MATLAB、C#、LabVIEW、CVI 等，不应把 Rust/Go 直接宣称为官方首选路径。
5. USB/GPIB/HiSLIP/VXI-11 后续可能需要 VISA，但首版不需要提前引入 FFI 复杂度。

## Decision

SMB100A 的 first implementation 采用：

```text
SCPI over LAN raw socket
```

默认连接方式：

```text
TCP socket
host = SMB100A IP address or hostname
port = 5025
terminator = \n
encoding = ASCII / UTF-8 compatible SCPI text
```

首版执行路径：

```text
resolved_recipe.json
  -> executor
  -> smb100a driver
  -> TCP socket
  -> SCPI command
  -> SMB100A
```

Python RsInstrument 保留为：

```text
validation path
fallback path
manual debug path
cross-check path
```

VISA FFI 暂缓，直到出现以下需求：

```text
USB 控制
GPIB 控制
HiSLIP / VXI-11 的 device locking / SRQ / richer IEEE 488.2 behavior
必须接入实验室已有 VISA 资源管理体系
```

Rust 和 Go 被视为 technically possible 的 socket client 实现语言，但不作为“R&S 官方 first-class 示例目标”来宣传。

## Scope

本 ADR 只规定 SMB100A 的首版远程控制技术路径。

覆盖范围：

```text
SMB100A 连接方式
SCPI socket client
命令发送与查询
错误检查
超时处理
dry-run 日志
RsInstrument fallback
VISA FFI 延后条件
```

不覆盖：

```text
OE1022D 采集
磁场控制
Laser 控制
recipe schema 的完整定义
RF 安全限制的具体数值
GUI 页面细节
```

## Driver Interface

首版 SMB100A driver 需要暴露稳定接口，而不是让上层模块直接拼接 socket 字符串。

建议接口：

```text
connect(profile) -> connection
query(command) -> response
write(command) -> ack / error
check_error() -> instrument_error_state
apply_plan(scpi_plan) -> execution_report
close() -> result
```

上层 executor 只调用 driver 接口，不直接操作 TCP socket。

## Device Profile Example

```json
{
  "device_id": "smb100a_main",
  "kind": "smb100a",
  "transport": {
    "type": "tcp_scpi_socket",
    "host": "192.168.1.120",
    "port": 5025,
    "terminator": "\n",
    "connect_timeout_ms": 3000,
    "read_timeout_ms": 3000,
    "write_timeout_ms": 3000
  },
  "identity": {
    "idn_query": "*IDN?",
    "expected_vendor_contains": "Rohde"
  },
  "safety_profile": "smb100a_default_lab_limit"
}
```

## Initial SCPI Smoke Test

每次连接 SMB100A 后，首版 driver 必须至少执行：

```text
*IDN?
SYST:ERR?
OUTP?
FREQ?
POW?
```

目的：

```text
确认连接的是目标仪器
确认仪器错误队列可读
确认 RF 输出状态可读
确认频率和功率读数可读
```

除非处于显式 dry-run，否则连接失败、IDN 不匹配、读取超时、错误队列异常，均必须阻止实验执行。

## Safety Rules

SMB100A driver 必须遵守系统 safety 层，而不是自己绕过 safety。

必须满足：

```text
AI 不直接发送 SCPI
GUI 不直接发送 SCPI
未通过 safety_report 的 recipe 不得执行
RF ON / MOD ON / 大功率 / 扫频启动必须在 dry-run 中可见
所有发送到仪器的 SCPI 必须写入 events.jsonl
所有实际仪器响应必须记录
```

对危险命令的处理：

```text
OUTP ON
MOD:STAT ON
FREQ:MODE SWE
POW <value>
FM:STAT ON
SWE:MODE AUTO
```

这些命令必须经过 safety gate，并且必须在 dry-run 中展示。

## Error Handling

首版必须实现：

```text
connect timeout
read timeout
write timeout
unexpected disconnect
invalid response
instrument error queue not empty
IDN mismatch
command rejected by safety layer
```

建议策略：

```text
写命令失败 -> 立即停止当前 step
查询命令超时 -> 重试一次，然后失败
IDN mismatch -> 禁止执行
SYST:ERR? 返回非零错误 -> 记录并停止当前 step
RF 输出相关命令失败 -> 尝试 safe shutdown
```

safe shutdown 优先命令：

```text
OUTP OFF
MOD:STAT OFF
```

如果 safe shutdown 命令也失败，必须记录为 critical event，并提示人工检查仪器状态。

## Alternatives Considered

### Alternative 1: Python RsInstrument 作为唯一首版实现

优点：

```text
R&S 官方生态更成熟
调试方便
文档和示例更多
适合快速验证 SCPI 命令
```

缺点：

```text
会让实验执行路径依赖 Python runtime
不利于 Rust executor 的设备资源租约、事件日志和错误处理统一管理
可能重新形成 Python 脚本直接控制硬件的旧结构
```

结论：

```text
不作为唯一首版实现。
保留为 validation / fallback / debug path。
```

### Alternative 2: VISA / HiSLIP / VXI-11 作为首版实现

优点：

```text
协议能力更完整
HiSLIP / VXI-11 更接近仪器控制标准路径
可能支持 device locking、SRQ、remote/local 等更丰富行为
```

缺点：

```text
需要安装和维护 VISA runtime
跨平台部署复杂度上升
Rust FFI / binding 成本较高
首版只做基本 SCPI 控制时收益不明显
```

结论：

```text
Postponed。
等 USB/GPIB/HiSLIP/VXI-11 明确需要时再引入。
```

### Alternative 3: Rust raw socket first

优点：

```text
与 Rust executor 集成直接
最小外部依赖
容易统一事件日志、resource lease、error handling
```

缺点：

```text
R&S 官方 Rust 示例不是 first-class 路径
需要自己处理 terminator、timeout、error query、response parsing
没有 VISA 的高级仪器控制语义
```

结论：

```text
Accepted。
Rust socket driver 作为首版主路径。
```

### Alternative 4: Go raw socket first

优点：

```text
TCP socket 实现简单
部署方便
适合写轻量服务
```

缺点：

```text
当前系统核心已经倾向 Rust executor
引入 Go 会增加运行时和模块边界
R&S 官方 Go 示例不是 first-class 路径
```

结论：

```text
技术上可行，但首版不采用。
```

## Consequences

正面后果：

```text
SMB100A 控制路径依赖最小
不需要首版引入 VISA FFI
Rust executor 可以统一管理设备连接、事件日志和错误处理
dry-run 可以直接展示最终 SCPI 命令
Python RsInstrument 仍可用于交叉验证
```

负面后果：

```text
raw socket 不自带 VISA 的 device locking / SRQ 等高级机制
需要自己实现 timeout、重试、错误队列读取和 safe shutdown
多客户端同时连接风险需要通过 resource lease 和 UI 禁止策略管理
```

## Implementation Notes

首版实现建议分为三层：

```text
smb100a-scpi-plan
  负责把 resolved step 转成 SCPI command list

smb100a-socket-client
  负责 TCP connect / write / query / timeout / close

smb100a-driver
  负责 safety gate / command logging / error checking / execution report
```

不要让 GUI、AI agent、recipe compiler 直接调用 socket client。

## Test Requirements

必须提供：

```text
fake SMB100A SCPI socket server
*IDN? smoke test
SYST:ERR? error queue test
timeout test
disconnect test
invalid response test
OUTP ON safety rejection test
SCPI dry-run snapshot test
RsInstrument cross-check script
```

验收标准：

```text
没有真实 SMB100A 时，fake server 可以跑完整 dry-run 和 integration test
连接真实 SMB100A 时，可以稳定完成 IDN / freq / power / output state 查询
RF OFF 状态下可以配置频率、功率、LF、FM、sweep 参数
危险命令未经 safety gate 不会发送
所有已发送命令和仪器响应都能在 events.jsonl 中回溯
```

## References

- Rohde & Schwarz, R&S SMB100A Operating Manual, Version 23. See sections on Remote Control Interfaces and Protocols, LAN Interface, Socket Communication, and SCPI command reference.
- Rohde & Schwarz, Remote Control via SCPI - Getting Started, Version 04.
- Rohde & Schwarz, RsInstrument Python documentation and GitHub repository.
- Rohde & Schwarz, Remote Control and Instrument Drivers page.

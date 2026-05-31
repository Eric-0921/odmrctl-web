# ADR-007: SMB100A `SYST:ERR?` 返回 `-410,"Query interrupted"` 的处理决策

## Status

Accepted

## Date

2026-05-31

## Decision Owner

ODMR Automation Project

## Related Documents

```text
docs/prd/00_main_prd_v0.2.md
docs/prd/10_safety_interlock_prd_v0.2.md
docs/lab-bringup/m2_8_timeline_alignment_plan.md
docs/adr/ADR-003-smb100a-scpi-lan-socket.md
docs/adr/ADR-004-no-ai-live-hardware.md
docs/equipment_manual/smb100a/06a_common_commands.md
docs/equipment_manual/smb100a/06n_system_subsystem.md
docs/equipment_manual/smb100a/05_remote_control_basics.md
```

---

## 1. Context

在 M2.8 实验室 bring-up 中，我们对 SMB100A 执行了 query-only 状态快照（14 条 query 序列）。无论采用何种 delay / drain 策略，**`SYST:ERR?` 始终返回 `-410,"Query interrupted"`**。

复现条件：
- SMB100A 通过 TCP socket (LAN) 连接
- 查询序列：`*IDN?`, `OUTP?`, `MOD:STAT?`, `FREQ?`, `POW?`, `POW:ALC?`, `FM:STAT?`, `FM:SOUR?`, `FM:DEV?`, `LFO?`, `LFO:FREQ?`, `LFO:VOLT?`, `LFO:SHAP?`, `SYST:ERR?`
- 每条 query 后等待 100ms（`--smb-query-delay-ms = 100`）
- 每条 query 后执行 drain（读取 TCP 缓冲区残余数据）
- 100ms delay 后发送下一条 query

即使在上述保守策略下，`-410` 依然复现。

---

## 2. Problem Analysis

### 2.1 `-410` 是什么

根据 SCPI 标准（IEEE 488.2 §6.3.2.7）和多家仪器厂商文档（Tektronix、Keithley、Keysight、R&S）：

> **`-410, "Query INTERRUPTED"`**：发送了一个合法 query 后，在仪器**尚未完成发送整个响应消息（包括 line-feed/EOI terminator）之前**，又发送了新的命令或 query。仪器认为前一个 query 被"中断"了。

具体触发场景：
1. 发送 query A → 仪器开始准备响应
2. 在响应完全发送之前，发送了 query B
3. 仪器将 query A 标记为 "interrupted"
4. 错误码 `-410` 被写入错误队列

### 2.2 为何 drain + delay 未能消除

当前代码做了两层防护：

```
1. read_line() 读到换行符为止
2. drain_buffer() 用非阻塞 read 清空 TCP 接收缓冲区
3. sleep(100ms)
4. 发送下一条 query
```

但 `-410` 依然出现，原因可能是：

| 可能原因 | 解释 |
|---------|------|
| **BufReader 预读取** | `BufReader::read_line()` 可能在内部 buffered 了超出单条响应的数据，导致 `self.stream.read()` 在 drain 时看不到残余数据 |
| **仪器响应尚未完全发出** | SMB100A 的 SCPI 处理器在 `read_line()` 返回后，可能仍在内部 buffer 中持有响应尾部的换行符/EOI，100ms 不足以保证其完成发送端 flush |
| **TCP 流的半双工时序** | 即使客户端 drain 了接收缓冲区，仪器端的 TCP 栈可能尚未完成发送 handshake，导致仪器认为响应还在"进行中" |
| **EOI 未正确识别** | SCPI 标准要求响应以 line-feed + EOI 结束。`read_line()` 读到 `\n` 即返回，可能未等待 EOI 信号 |

> 注：这是 SCPI 标准中"最频繁出现的三个错误之一"（Tektronix 应用笔记），属于通信时序问题，**不是硬件故障**。

### 2.3 为何 `SYST:ERR?` 最后执行反而复现

M2.8 已将 `SYST:ERR?` 移至查询序列末尾（在所有常规 query 之后）。此时：
- 之前的 13 条 query 已积累了 13 次潜在中断
- `SYST:ERR?` 读取的是错误队列中的**最老条目**
- 返回 `-410` 意味着这些中断已被仪器记录在案

换句话说：**`-410` 不是 `SYST:ERR?` 本身触发的，而是前面 13 条 query 的序列行为触发的**。`SYST:ERR?` 只是"揭露"了已存在的问题。

---

## 3. 可选方案

### 方案 A：查询序列开头发送 `*CLS`

在 `*IDN?` 之前发送 `*CLS`（Clear Status）。

> `*CLS`：Sets the status byte (STB), the standard event register (ESR) and the EVENT part of the QUESTionable and the OPERation registers to zero. **It clears the output buffer.**

- **优点**：一次性清空错误队列和输出缓冲区，确保后续查询从一个干净状态开始
- **缺点**：`*CLS` 不是 query（是 setting command），在 M2.8 的严格 query-only 模式下需要安全审查
- **安全评估**：`*CLS` 不修改仪器输出状态（频率/功率/开关状态均不变），只清除寄存器和缓冲区。从物理安全角度是"无害的"

### 方案 B：在 `SYST:ERR?` 后继续 drain 直到 `0,"No error"`

发送 `SYST:ERR?` 后，若返回非零错误码，循环发送 `SYST:ERR?` 直到错误队列为空。

- **优点**：自动清除所有历史错误，不引入 `*CLS`
- **缺点**：如果 `-410` 持续产生，可能进入无限循环；每次 `SYST:ERR?` 本身也可能触发新的 `-410`

### 方案 C：改进 drain 策略（带超时确认）

将当前 `drain_buffer()` 改为带超时的"确认 drain"：

1. 发送 query → `read_line()` 读到换行符
2. 设置一个短超时窗口（如 200ms）
3. 持续非阻塞 read，直到连续 N 次返回 0 字节或 `WouldBlock`
4. 确认仪器已完成发送后，再发下一条 query

- **优点**：从根因上减少 `-410` 产生
- **缺点**：增加查询序列总时长；200ms 的"静默确认"逻辑复杂；TCP 流在半双工场景下可能仍无法 100% 保证

### 方案 D：接受 `-410` 为预期行为，标记为 warning（当前方案）

保持 M2.8 现有策略：
- `-410` 被记录为 `station_snapshot_quality.warnings`
- `eligible_for_rf_on_microtest = false`
- 不导致 M2.8 run 失败
- 在 RF ON 微测试前（M2.9 或 M3）再统一清空错误队列

- **优点**：不修改查询序列，不改变安全边界，M2.8 目标（timeline alignment + passive acquisition）不受影响
- **缺点**：错误队列未清空，无法进入 RF ON 阶段；`-410` 会在每次 run 中复现

---

## 4. 决策

**决定：在 M2.8 中采用方案 D（接受 `-410` 为预期 warning）。**

理由：

1. **M2.8 的目标是 timeline alignment 和 passive acquisition**，不是 RF ON。错误队列是否清空不影响 M2.8 的核心交付物。
2. **`-410` 是通信时序副作用，不是硬件故障**。所有查询结果（`OUTP?`→`0`, `FREQ?`→`2882000000`, `POW?`→`-15` 等）均正确。
3. **M2.8 的安全边界是 "query-only"**。引入 `*CLS`（方案 A）虽然是"无害的"，但它是一个 setting command，会模糊 query-only 的严格边界。保持边界清晰对安全审查更有价值。
4. **方案 C 的收益有限**。即使增加 drain 超时，`-410` 仍可能因 TCP 栈时序而不可预测地复现。投入产出比不高。
5. **`eligible_for_rf_on_microtest = false`** 已经正确反映了"错误队列未清空，不能进入 RF ON"的状态。这个 safety gate 是有效的。

**M2.9 或 M3 的过渡条件**：
- 在进入任何 RF ON 微测试之前，必须先发送 `*CLS` 或循环 `SYST:ERR?` 直到返回 `0,"No error"`
- 这是一个明确的 pre-condition，而非 M2.8 的 blocker

---

## 5. 相关设备参数

### SMB100A 错误队列行为

根据 `docs/equipment_manual/smb100a/`：

> "Each error state in the instrument leads to an entry in the error queue. The entries of the error queue are detailed plain text error messages... Each call of `SYSTEM:ERROR[:NEXT]?` provides one entry from the error queue. If no error messages are stored there any more, the instrument responds with `0, 'No error'`."

> "The error queue should be queried after every SRQ in the controller program as the entries describe the cause of an error more precisely than the status registers."

### `*CLS` 行为

> "Sets the status byte (STB), the standard event register (ESR) and the EVENT part of the QUESTionable and the OPERation registers to zero. The command does not alter the mask and transition parts of the registers. **It clears the output buffer.**"

Usage: **Setting only**（非 query）

### SCPI 标准对 `-410` 的定义

IEEE 488.2 §6.3.2.7：
> "A condition causing an INTERRUPTED Query error occurred. For example, a query was followed by DAB or GET before a response was completely sent."

---

## 6. Consequences

### 6.1 Positive consequences

- M2.8 安全边界保持清晰（strict query-only）
- 不引入额外的 setting command 到 M2.8 的 allowlist
- `eligible_for_rf_on_microtest = false` 正确阻止了在错误队列未清空时进入 RF ON
- 所有查询数据仍然正确可用

### 6.2 Negative consequences

- 每次 M2.8 run 都会在 `station_snapshot_quality.json` 中产生一个 warning
- `SYST:ERR?` 无法用于验证"无其他隐藏错误"（因为 `-410` 掩盖了队列中可能存在的其他错误）
- 需要在未来 milestone 中显式处理错误队列清空

---

## 7. Future migration path

### M2.9（RF ON shadow mode）

在 M2.9 中，当需要验证 RF ON 链路时，查询序列应扩展为：

```
1. *CLS                    ← 新增：清空错误队列和输出缓冲区
2. *IDN?
3. OUTP?
4. ...（常规查询序列）
5. SYST:ERR?
6. 若 SYST:ERR? 返回非零，循环执行 SYST:ERR? 直到 0,"No error"
7. 此时错误队列为空，eligible_for_rf_on_microtest 可设为 true
```

`*CLS` 在 M2.9 中的引入需要单独的 safety review，但其"不修改仪器输出状态"的特性使其在安全上可接受。

### M3（Full executor）

executor 的 device initialization 阶段应包含：
- 发送 `*CLS`
- 验证 `SYST:ERR?` 返回 `0,"No error"`
- 将错误队列状态作为 `DeviceStatus` 的一部分报告

---

## 8. Acceptance criteria

本 ADR 被满足的条件：

1. M2.8 代码不修改 SMB100A 查询序列（保持 strict query-only）
2. M2.8 代码继续将 `-410` 记录为 warning（不升级为 error）
3. `eligible_for_rf_on_microtest` 在 `-410` 存在时为 `false`
4. M2.8 的 timeline alignment 和 passive acquisition 功能不受影响
5. M2.9 planning 文档中明确包含错误队列清空作为 RF ON pre-condition

---

## 9. Decision summary

`-410, "Query interrupted"` 是 SMB100A 对快速连续 query 的通信时序反馈，属于 SCPI 标准行为，不是硬件故障。

M2.8 的核心目标是 timeline alignment + passive acquisition，不是 RF ON。因此，**接受 `-410` 作为预期 warning 是合理的**，它通过 `eligible_for_rf_on_microtest = false` 正确地阻止了不安全的 RF ON 操作。

错误队列的清空将推迟到 M2.9 或 M3，作为进入 RF ON 阶段的显式前置条件。

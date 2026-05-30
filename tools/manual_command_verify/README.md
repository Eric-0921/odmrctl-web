# Manual Command Verification Protocol

## 目的

本目录提供 **Human-in-the-Loop** 的手动命令验证协议，用于在真实硬件上逐条验证 SMB100A 和 OE1022D 的 SCPI / ASCII 命令。

**核心原则**:
- 查询命令（Query Phase）可自动执行
- 设置命令（Set Phase）必须经操作员显式确认后才能发送
- 危险命令被硬编码拦截，脚本永远不发 `OUTP ON`、`MOD:STAT ON`、`FM:STAT ON`、`FREQ:MODE SWE`、`SWE:EXEC`

## 文件结构

| 文件 | 说明 |
|------|------|
| `docs/lab-bringup/smb100a_command_verification.md` | SMB100A 验证协议文档 |
| `docs/lab-bringup/oe1022d_command_verification.md` | OE1022D 验证协议文档 |
| `examples/verification/smb100a_observed_responses.jsonl` | SMB100A 观测记录模板 |
| `examples/verification/oe1022d_observed_responses.jsonl` | OE1022D 观测记录模板 |
| `scripts/lab/verify-smb100a.sh` | SMB100A 交互式验证脚本 |
| `scripts/lab/verify-oe1022d.sh` | OE1022D 交互式验证脚本 |

## 使用流程

### 1. 阅读协议文档

先阅读对应的 Markdown 协议，了解两阶段验证流程：
- **Phase 1 (Safe Query)**: 只读查询，确认设备身份和当前状态
- **Phase 2 (Safe Set)**: 安全设置，仅在人工确认后执行

### 2. 运行交互式脚本

```bash
# SMB100A (TCP 5025)
bash scripts/lab/verify-smb100a.sh 169.254.2.20 5025

# OE1022D (Serial)
bash scripts/lab/verify-oe1022d.sh /dev/cu.usbmodem3361358734371 115200
```

脚本行为：
- **查询阶段**：自动发送，记录响应
- **设置阶段**：显示命令和安全上下文，提示 `Execute? (y/n)`
- **危险命令**：若检测到禁止字符串，立即报错退出

### 3. 填写 JSONL 记录

脚本会自动追加到 `examples/verification/*.jsonl`。操作员可在 `human_notes` 字段补充观察。

### 4. 人工审核

验证完成后，人工检查：
- 每个命令都有 `observed_response` 或 `timeout`
- 所有失败都有 `human_notes` 说明
- `pass_fail` 字段已填写

## 安全门

脚本内置硬编码禁止列表：

```text
OUTP ON
MOD:STAT ON
FM:STAT ON
FREQ:MODE SWE
SWE:EXEC
```

任何匹配上述字符串的命令都会被拒绝发送。

## ADR-004 合规

本协议严格遵循 ADR-004：
- AI 不直接控制 live hardware
- 所有 setter 命令必须经过人类操作员显式批准
- 验证产物以文件形式保存，可追溯、可审计

# SMB100A 命令验证协议

> **Human-in-the-Loop Protocol**
> 本协议要求操作员逐条确认设置命令。AI/脚本不会自动发送任何改变设备状态的命令。
> 危险命令（`OUTP ON`、`MOD:STAT ON`、`FM:STAT ON`、`FREQ:MODE SWE`、`SWE:EXEC`）被硬编码禁止。

---

## 验证环境

| 项目 | 值 |
|------|-----|
| Device | R&S SMB100A |
| Transport | TCP SCPI, 169.254.2.20:5025 |
| MAC | 00:90:b8:1f:06:dd |
| Operator | （填写操作员姓名） |
| Date | （填写日期） |
| Safety Check | RF 输出已关闭 / 已接安全负载 |

---

## Phase 1: Safe Query Phase（只读查询，无需人工确认）

> 本阶段所有命令均为查询，不会改变设备状态。脚本可自动执行。

| # | Command | Expected Response | Observed Response | Pass/Fail | Timestamp | Human Notes |
|---|---------|-------------------|-------------------|-----------|-----------|-------------|
| 1 | `*IDN?` | `Rohde&Schwarz,SMB100A,...` | | | | |
| 2 | `SYST:ERR?` | `0,"No error"` | | | | |
| 3 | `OUTP?` | `0` (OFF) | | | | |
| 4 | `MOD:STAT?` | `0` (OFF) | | | | |
| 5 | `FREQ?` | `~2.882E9` | | | | |
| 6 | `POW?` | `~-15` | | | | |
| 7 | `POW:ALC?` | `AUTO` | | | | |
| 8 | `FM:STAT?` | `0` (OFF) | | | | |
| 9 | `FM:SOUR?` | `INT` 或 `EXT` | | | | |
| 10 | `FM:DEV?` | `~4E6` (若 FM 曾配置) | | | | |
| 11 | `LFO?` | `1` (ON) 或 `0` (OFF) | | | | |
| 12 | `LFO:FREQ?` | `~500` | | | | |
| 13 | `LFO:VOLT?` | `~0.137` | | | | |
| 14 | `LFO:SHAP?` | `SQUARE` | | | | |
| 15 | `FREQ:MODE?` | `CW` | | | | |

---

## Phase 2: Safe Set Phase（安全设置，需人工逐条确认）

> ⚠️ **重要**：执行本阶段前，操作员必须确认：
> 1. SMB100A 的 RF 输出端口已断开或已接安全负载
> 2. 周围人员已知晓验证正在进行
> 3. 每条 setter 命令发送前，脚本会提示 `Execute? (y/n)`

| # | Command | Purpose | Expected Display | Observed Response | Pass/Fail | Approved By | Timestamp | Human Notes |
|---|---------|---------|------------------|-------------------|-----------|-------------|-----------|-------------|
| 1 | `OUTP OFF` | 确保 RF 输出关闭 | RF OFF | | | | | |
| 2 | `MOD:STAT OFF` | 关闭调制总开关 | MOD OFF | | | | | |
| 3 | `FREQ:MODE CW` | 设为固定频率模式 | CW | | | | | |
| 4 | `FREQ 2.882GHz` | 设置频率 2.882 GHz | 2.882 GHz | | | | | |
| 5 | `POW -15dBm` | 设置功率 -15 dBm | -15.00 dBm | | | | | |
| 6 | `POW:ALC AUTO` | ALC 自动模式 | ALC-Auto | | | | | |
| 7 | `FM:STAT OFF` | 关闭 FM 调制 | FM OFF | | | | | |

---

## Forbidden Commands（本协议禁止发送）

以下命令在本次验证协议中**绝对禁止**发送。若脚本检测到这些字符串，会立即报错退出：

| Command | 危险原因 |
|---------|----------|
| `OUTP ON` | 打开 RF 微波输出 |
| `MOD:STAT ON` | 打开模拟调制总开关 |
| `FM:STAT ON` | 打开频率调制 |
| `FREQ:MODE SWE` | 进入扫频模式 |
| `SWE:EXEC` | 执行扫频 |

---

## 验证总结

| 项目 | 统计 |
|------|------|
| Query Phase 命令数 | 15 |
| Safe Set Phase 命令数 | 7 |
| 通过数 | （填写） |
| 失败数 | （填写） |
| 跳过数 | （填写） |
| 禁止命令触发次数 | 应为 0 |

**操作员签字**: _______________  
**审核员签字**: _______________

# OE1022D RALL? 高速读取 — 决策修正

**Date**: 2026-06-06
**Status**: Accepted (supersedes `oe1022d-high-speed-buffer-acquisition.md`)

## 背景

`oe1022d-high-speed-buffer-acquisition.md` 在以下错误前提下撰写：

1. RALL? 每帧读取需要 ~1 秒，只能做到 ~1 fps
2. 因此 RALL? 无法支撑 1 kHz 连续采集
3. 需要走 Buffer 子系统 + TRCAD? 路径

经真实硬件验证，以上前提全部错误。本文件为决策修正。

## 实测结论

### RALL? 真实性能

| 指标 | 旧认知（错误） | 实测（正确） |
|------|--------------|------------|
| 单帧读取时间 | ~800-1000ms | **47-49ms** |
| 帧率 | ~1 fps | **20.5 fps** |
| 数据点/秒 | ~50 | **~1000** |
| USB 吞吐 | 12 KB/s | **~247 KB/s** |
| 新数据占比 | 未知 | **96-98%**（几乎每帧更新） |

瓶颈原因：Rust 代码中 `sleep(800ms)` 在发送 RALL? 后、开始读数据前（`oe1022d_acquire/src/main.rs:267`、`executor_shadow_run/src/main.rs:772`、`rf_mag_oe_minimal_run/src/oe_bridge.rs:67`）。

修复方式：移除固定 sleep，改为 1-3ms 快速轮询，满 12288 字节后立即返回。

### RALL? 的工作原理

- 设备内部每 **50ms** 生成一帧 12288 字节二进制数据
- 每帧包含 **20 个参数 × 50 个时间点**（点间距 1ms）
- USB CDC 传输 12288 字节约需 47-49ms
- 连续逐帧读取每秒可获得 ~1000 个数据点 = **1 kHz 等效**

手册原文完全一致：「返回数据每 50ms 更新一次，数据采样间隔是 1ms，每次返回之前的 50 个测量数据」。

### TRCAD? 不存在

- `TRCAD?` 在 OE1022D 固件 V6.3211110 上**不存在**（230+ 组合暴力测试证实）
- `TRCA?` 同样不存在
- `TRCB?` 同样不存在（LabVIEW 源码中的 `TRCB?%d,%d,%d` 是 OE1022 前代型号的命令，OE1022D 未实现）
- Buffer 子系统配置命令（SRATD/SLEND/SSLED/STRGD/SPRMD/STRDD/PAUSD/RESTD/SPTSD?）全部正常工作，但**数据无法读出**

### LabVIEW 源码分析

SSI 官方 LabVIEW 源码中：
- OE1022D 的数据读取**就是 RALL?**，没有其他路径
- `OE1022D_USB_Query Data.vi` → `SSI_Command.vi` → `SSI_Control.vi` → VISA Read → `OE1022D_DATA Transmit.vi`（解析）
- 「1kHz 实时图表」就是连续 RALL? 达到的 20fps × 50 点效果
- `TRCB?%d,%d,%d` 仅出现在 OE1022（前代）示例中，OE1022D 用不了

## 决策

### 决策 1：RALL? 是 OE1022D 的唯一数据读取路径

经硬件验证和 LabVIEW 源码双向确认，OE1022D 固件 V6.3211110 上**只有 RALL? 可以读取测量数据**。Buffer 子系统可以配置、可以填充，但没有可用的数据读出命令。

### 决策 2：撤销 Buffer/TRCAD 路径

`oe1022d-high-speed-buffer-acquisition.md` 中关于 TRCAD? 和 Buffer 乒乓方案的规划**全部撤销**。原因：
- 硬件不支持，不存在 TRCAD? 命令
- Buffer 子系统仅可用于内部配置，无法作为数据读出路径

### 决策 3：RALL? 作为 1 kHz 采集的唯一主路径

连续 RALL? 读取可以达到 ~20 fps / ~1000 点/秒，满足项目 1 kHz 需求。

注意事项：
- **帧边界**：每帧前必须 `clear(Input)` 或保证上次读取完整，防止帧移位
- **重复检测**：读得比 50ms 刷新快时会拿到重复帧，需比较 X[0] 或整帧去重
- **时间戳**：帧内点间距 1ms 是确定的，但帧的绝对时间戳来自 PC 时钟
- **无帧头标识**：12288 字节纯二进制无帧头帧尾，依赖长度对齐

### 决策 4：读取使用快速轮询模式

禁止在 RALL? 写操作后使用超过 5ms 的固定 sleep。标准模式：

```
write("RALL?\r") → flush() → loop { read(4K buf); if total >= 12288: break; sleep(1-3ms) }
```

## 影响范围

需要修改的代码（已完成）：
- `tools/lab/oe1022d_acquire/src/main.rs` — 移除 800ms sleep
- `tools/lab/executor_shadow_run/src/main.rs` — 移除 800ms sleep
- `tools/lab/rf_mag_oe_minimal_run/src/oe_bridge.rs` — 移除 frame_delay_ms sleep
- `tools/lab/oe1022d_buffer_probe/src/rall_detailed.rs` — 移除 200ms sleep

需要更新的文档：
- `docs/lab-bringup/oe1022d_buffer_1khz_validation_final_report.md` — 补充修正章节
- `docs/decisions/oe1022d-high-speed-buffer-acquisition.md` — 标注为已废弃

不需要的代码：
- `odmr-oe1022d/src/commands.rs` 中的 TRCAD? builder 保留但标注为「固件不支持」
- Buffer 命令 builder 保留（配置命令仍可用），但 TRCAD 相关的 executor/logging 规划不再进行

## 性能预算

| 参数 | 值 |
|------|-----|
| 单帧字节数 | 12288 |
| USB CDC 包大小 | ~1020 字节 |
| 帧传输包数 | 12 |
| 单帧 wall-clock | 47-49ms |
| 帧率 | ~20.5 fps |
| 数据点/秒 | ~1000 |
| USB 带宽占用 | ~247 KB/s（921600 baud 理论 90 KB/s，实测超额说明 CDC 实际带宽高于串口波特率限制） |

## 联系厂商

建议联系 SSI 确认：
1. TRCAD? 是否在更新的固件版本中实现
2. 是否有固件升级计划为 OE1022D 加入 Buffer 数据读出支持

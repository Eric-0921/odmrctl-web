# OE1022D RALL? 连续采集基准测试

**Date**: 2026-06-06
**Status**: Accepted（基于真机实测）
**Supersedes**: `oe1022d-rall-fast-read-correction.md` 中关于 49ms/帧的结论

## 实测数据

硬件：OE1022D 固件 V6.3211110, SN:D6130220, USB CDC @ 921600
测试工具：`tools/lab/oe1022d_buffer_probe --rall-bench`
数据采集：2026-06-06 真机实跑

### Test A: 单帧读取时间（10 帧，每帧前 clear(Input)）

| 帧号 | 读取时间 | 字节数 | X[0] |
|------|---------|--------|------|
| 1 | 12.0ms | 12288 | 9.97e-7 |
| 2-10 | 11.8-12.0ms | 12288 | ... |

**平均**: 11.9ms/帧 → **83.7 fps 机械读取上限**

### Test B: 连续读取（无 clear，读完立即发下一个 RALL?）

- 20 帧耗时 239ms，帧间隔稳定 **12.0ms**
- 有效去重帧率: **20.8 fps**（6/20 帧为新数据，70% 重复）
- 设备每 **~48ms** 刷新一次数据帧（手册所述 50ms 吻合）

### Test C: Pipeline 模式（连发 5 个 RALL? 再读）

- 第 1 帧正常（11ms），第 2 帧只返回 832 字节垃圾
- 第 3-5 帧返回 0 字节
- **结论：设备不支持 RALL? 流水线，一次只能排队一个请求**

## 完整性能表

| 指标 | 之前认为 | 纠正后 | 最终实测 |
|------|---------|--------|---------|
| 单帧传输 | ~800ms（bug） | ~49ms | **12.0ms** |
| 最大 fps | ~1 | ~20.5 | **83.7** |
| 有数据 fps | ~1 | ~20.5 | **20.8** |
| pts/sec（有效） | ~50 | ~1000 | **~1040** |
| 重复率 @ 83fps | 未知 | 未知 | **70%** |
| Pipeline | 未知 | N/A | **不支持** |

## 决策

### 决策 1：采集节拍设为 ~48ms

读取 RALL? 的实际物理限制是设备刷新率（48-50ms），不是 USB 带宽。正确做法：
- 读取节拍 = 48ms（匹配设备刷新周期）
- 每次读取前 `clear(Input)` 防止帧边界漂移
- 不做 pipeline、不连发多个 RALL?

### 决策 2：去重检测放在采集层

因为设备刷新和数据读取有时间差，可能连续读到相同帧。去重策略：
- 比较帧头 X[0] 值（简单高效，X[0] 通常变化明显）
- 或比较前 64 字节的 hash
- 去重帧计为 `skipped_frame`，计入 index

### 决策 3："有效 1kHz"的含义

RALL? 提供 **48ms 间隔的 50 点 burst**：
- 帧内时间轴精确（硬件 1ms 采样间隔）
- 帧间时间轴由 PC 时钟时间戳维护
- 用户要求「任意采样率」时，从 50 点中抽取/聚合（不需要软件插值）

### 决策 4：不模仿 LabVIEW 的「任意采样率」

LabVIEW 的 `Sample Time(ms)` + `Wait(ms)` 组合实际上改变的是硬件 Buffer 配置（SRAT 索引）和 PC 轮询间隔。
我们没有 TRCB? 能力（固件不支持），因此 PC 端可变的只有 RALL? 的读取频率。
用户需要的采样率通过**帧内点抽取 + 帧间时间戳对齐**实现。

## 与 LabVIEW 的对比

| 维度 | LabVIEW | 我们的方案 |
|------|---------|-----------|
| 数据源 | OE1022D: RALL?, 非D版: TRCB? | RALL?（TRCB? 不可用） |
| 传输层 | VISA Serial | serialport crate |
| 图表架构 | Queue (Producer-Consumer) | mpsc channel + 独立渲染线程 |
| 数据格式 | OE1022D_DATA Transmit.vi 解析 | odmr_oe1022d::parser::parse_rall_frame |
| 去重 | Formula Box (now_point vs old_point) | X[0] 值比较 |
| 保存 | Write To Spreadsheet File.vi（条件） | odmr-logging raw bin + jsonl |
| 插值/重采样 | 无 | 帧内点抽取/聚合 |

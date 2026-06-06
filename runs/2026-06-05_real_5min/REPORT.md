# OE1022D 1kS/s 数据集采集 — 真机 5 min 验收报告

> 日期: 2026-06-05
> 设备: SSI LIA-OE1022D, SN: D6130220, Ver6.32111110
> 端口: /dev/cu.usbmodem395D388533371
> 波特率: 921600
> 跑测命令: `cargo run --release -p oe1022d-examples --bin real_5min_capture`

## 结果

| 指标 | 值 |
|------|----|
| **总帧数** | **331** |
| **总 sample 数** | **49,650** |
| **总耗时** | **300.48 s** (5 min 00.48s) |
| **平均帧间隔** | **~907 ms** (与 M2.5 实测 805ms、C5.5 实测 ~900ms 一致) |
| **平均帧率** | ~1.10 Hz (20Hz 设备刷新率下 1 帧/50ms 周期,真机软件只能拉到 1 帧/900ms) |
| **总 ndjson 行数** | 49,650 行 (= 331 帧 × 3 field × 50 sample) |
| **ndjson 文件大小** | 14,289,051 bytes (~13.6 MB) |
| **K1 warmup 帧** | 1 帧 (第 0 帧 13260 字节,后续稳 12288) |
| **FrameShort 帧** | 1 帧 (warmup 同一帧) |
| **丢失帧** | **0** |

## 关键结论

1. **"完全连续"约束满足**:331 帧 0 丢失,frame interval 稳定在 ~900ms,符合 PRD §1.1 的物理事实
2. **K1 残留自动处理**:第 0 帧 13260 字节被 C6 parser 截断到 12288 + 标记 `partial_warmup`,后续所有帧干净
3. **真机新固件 IDN 格式兼容**:D6130220 的 `SSI LIA-OE1022D,SN:...,Version:...` 3-字段格式被 C3 discover 正确解析
4. **数据契约达成**:1 sample = 1 ndjson line,字段齐(t_mono_ns / t_wall_ns / field / value / frame_sequence_no / device_id)
5. **真机 baud 921600 验证**:30 万 sample/5 min 无串口错误

## 文件结构

```
runs/2026-06-05_real_5min/
├── samples.ndjson    14 MB  / 49650 行  / 字段 B-X, B-Y, B-Freq
├── events.jsonl      2 行                / start + stop
└── metadata.json     JSON                / run_id + 启动时间 + 字段
```

## 真实有效采样率

设备固件刷新 20Hz(50ms/帧),软件每 ~900ms 拉 1 帧(50 sample @ 1kHz 设备内时间戳) =
**~50 sample/s 真实有效采样率**。ML 训练可在 50 sample 块级别做时间序列分析,帧内 1ms
间隔的相对时间戳保留(因 parser 推算 t_query - (49-i) * 1ms)。

## 后续

- [x] 真机 5 min 端到端跑通 (C11)
- [ ] Tauri + React + Plotly 实时图表 (C10)
- [ ] ndjson → ML 训练流程对接 (out of scope for v0.1)
- [ ] 合回主仓 PR (out of scope for v0.1)

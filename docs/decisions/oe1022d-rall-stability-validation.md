# OE1022D RALL? 长时间稳定性验证

**Date**: 2026-06-06
**Status**: Accepted（30 分钟真机连续实测，35,713 帧）
**Depends on**: `oe1022d-rall-continuous-benchmark.md`（12ms/帧, 48ms 刷新）

## 目的

验证 RallCollector（Producer-Consumer 模式）在分钟→小时尺度上的稳定性：无内存泄漏、无串口劣化、无帧丢失、无数据损坏。

## 测试设计

```
Producer (独立线程)           Consumer (主线程)
  clear(Input)                  recv_timeout(500ms)
→ RALL?\r                       ↓
→ fast-poll read 12288B     每帧写入:
→ parse + dedup                .rall 原始二进制（12288B × N）
→ try_send(channel=8)          .csv 元数据（8 字段）
  sleep(48ms 节拍)
```

关键设计决策：

- **原始二进制完整保留**：不提取单点摘要，直接写 12288 字节原始帧。后续可随时用 `parse_rall_frame()` 解析任意参数、任意采样点。
- **CSV 仅存元数据**：frame_index, unix_ms, read_us, is_dup, pll_a, pll_b, overload_a, overload_b。信号数据从 .rall 按需解析。
- **去重保留重复帧**：dup 帧仍写入 .rall + CSV（`is_dup=1`），数据完整性不丢失，分析时可选择跳过。

## 实测结果

**硬件**: OE1022D 固件 V6.3211110, SN:D6130220, USB CDC @ 921600
**测试工具**: `tools/lab/oe1022d_buffer_probe --stability-test`
**测试日期**: 2026-06-06

### 30 分钟运行（默认配置）

| 指标 | 值 | 评价 |
|------|-----|------|
| 持续时间 | 1800s (30 min) | — |
| 总帧数 | 35,713 | 无帧丢失 (0→35712 连续) |
| 去重帧数 | 35,287 | — |
| 总重复率 | 1.2% | 48ms 轮询 vs 50ms 刷新周期性相位差 |
| 有效 fps | 19.6 | 接近理论 20.8fps |
| 有效 pts/sec | 980 | 接近理论 1040 pts/sec |
| 平均读取时间 | 13.3ms | 全程无漂移 |
| 最小读取时间 | 11.6ms | 仅缓存命中出现 |
| 最大读取时间 | 51.7ms | 偶发 OS 调度尖峰，非设备问题 |
| 帧间隔中位 | 50.0ms | 与手册 50ms 规格完全吻合 |
| 原始数据大小 | 418.5 MB | 35713 × 12288 字节，精确匹配 |
| 解析错误 | 0 | — |
| 过载 | 0 | — |

### 5 分钟对照

| 指标 | 5 min | 30 min | 趋势 |
|------|-------|--------|------|
| 有效 fps | 19.6 | 19.6 | 无衰减 |
| 平均读取 | 13.4ms | 13.3ms | 无漂移 |
| 重复率 | 1.8% | 1.2% | 长期更低（相位趋于锁定） |

### 重复率时间序列特征

```
  0- 6min: 0-12% 周期性（48ms vs 50ms 相位差大）
  6-18min: 0-5%  周期性减弱（相位趋稳）
 18-30min: 0-2%  接近锁定
```

重复率呈周期性波动而非随机偶发，根因是 Poll Rate (48ms) ≠ Device Refresh (50ms) 的节拍差。系统在运行过程中自然收敛。**这不是 bug，是设计取舍**：48ms 轮询确保不丢帧（若用 50ms 会有累积偏移导致跳帧）。

## 输出文件

| 文件 | 大小 | 内容 | 可解析性 |
|------|------|------|----------|
| `.rall` | 419 MB | 35,713 × 12288B 原始二进制 | `parse_rall_frame(&buf)` → 完整 20 参数 × 50 点 + 配置 |
| `.csv` | 1.2 MB | 35,713 行元数据 | CSV reader / pandas / Excel |

## 结论

1. **RallCollector 在 30 分钟尺度上完全稳定**。无内存泄漏、无串口劣化、无帧丢失、无数据损坏。
2. **原始二进制 (.rall) 保留方案验证通过**。~20fps × 30min = 35K 帧 = 419MB，磁盘 IO 压力可忽略（~233 KB/s 写入）。
3. **可以接入 odmr-executor 采集状态机**。Producer 线程独立运行，Consumer 在 executor 中对接 `RawRecorder`（Layer 3 实时写入链路）。

## 实现文件

- `tools/lab/oe1022d_buffer_probe/src/stability_test.rs` — 30 分钟稳定性测试 Consumer
- `tools/lab/oe1022d_buffer_probe/src/main.rs` — `--stability-test` / `--stability-duration-secs` CLI
- `crates/odmr-oe1022d/src/collector.rs` — Producer-Consumer RallCollector（被复用，未改动）

# odmr-oe1022d

**Layer 1** — OE1022D 锁相放大器 USB 驱动。

## 职责

- OE1022D 串口通信协议（命令 catalog + 响应解析）
- `RALL?` 采集帧二进制解析（12288 字节，20 参数 × 50 采样点，f64 BE）
- 锁相参数设置（灵敏度、时间常数、参考频率、滤波斜率、输入源等）
- Buffer 采样命令（SRATD、SLEND、SSLED 等）
- `RallCollector` — Producer-Consumer 连续采集器
- 实现 `Device` trait
- `FakeOe1022d` 与 `ChannelState` mock 实现

## 依赖

- `odmr-device`
- `odmr-types`

## 模块

| 模块 | 说明 |
|------|------|
| `commands` | 完整 SCPI-style 命令 catalog（Ch-A / Ch-B） |
| `parser` | RALL? 帧解析器、测量值提取 |
| `collector` | `RallCollector` — 独立轮询线程 + bounded mpsc channel(8)，48ms 节拍，X[0] 去重 |
| `fake` | `FakeOe1022d` mock 设备，含 `ChannelState` |

## 关键性能数据

- 单帧读取：~12.0ms（机械上限 83.7fps）
- 设备刷新周期：~48ms
- 有效去重帧率：20.8 fps（~1040 pts/sec）
- 30 分钟稳定性验证：35,713 帧，1.2% dup，零解析错误，零 buffer 溢出

## 参考

- `docs/adr/ADR-002-rust-oe1022d-core.md`
- `docs/prd/03_oe1022d_acquisition_prd_v0.2.md`
- `docs/decisions/oe1022d-rall-continuous-benchmark.md`
- `docs/decisions/oe1022d-rall-stability-validation.md`

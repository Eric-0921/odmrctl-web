# odmr-oe1022d

**Layer 1** — OE1022D 锁相放大器 USB 驱动。

## 职责

- OE1022D 串口通信协议
- RALL? 采集帧二进制解析
- 锁相参数设置（灵敏度、时间常数、参考频率）
- 实现 `Device` trait
- 高频连续采集数据流

## 依赖

- `odmr-device`
- `odmr-types`

## 参考

- `docs/adr/ADR-002-rust-oe1022d-core.md`
- `docs/prd/03_oe1022d_acquisition_prd_v0.2.md`

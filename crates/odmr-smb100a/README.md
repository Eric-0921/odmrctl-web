# odmr-smb100a

**Layer 1** — SMB100A RF 信号源 SCPI 驱动。

## 职责

- SMB100A SCPI 指令封装（频率、功率、扫描模式、调制）
- 实现 `Device` trait（通过 LAN socket）
- 频率扫描参数设置与查询

## 依赖

- `odmr-device`
- `odmr-types`

## 参考

- `docs/adr/ADR-003-smb100a-scpi-lan-socket.md`
- `docs/prd/02_device_registry_connection_prd_v0.2.md`

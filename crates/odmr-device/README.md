# odmr-device

**Layer 1** — 设备抽象层。定义 Device trait、DeviceManager、ResourceLease。

## 职责

- `Device` trait（connect/disconnect/read/write 等通用接口）
- `DeviceManager`：设备注册表 + 生命周期管理
- `ResourceLease`：互斥访问某设备
- 连接状态机
- 设备发现

## 依赖

- `odmr-types`

## 不负责

- 具体设备的 SCPI 指令（那是 odmr-smb100a 的事）
- 串口协议细节（那是 odmr-oe1022d 的事）
- 硬件型号硬编码

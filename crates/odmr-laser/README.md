# odmr-laser

**Layer 1** — CNI laser PSU-SR binary serial driver.

## 职责

- CNI 激光器二进制串口协议帧封装（`HEADER [0x55, 0xAA]`）
- `LaserFrame`：功率设置、激光开关、帧序列化/反序列化
- `LaserClient`：串口打开、功率设置、开关控制、紧急关闭、echo 身份检查
- 手册硬上限：最大激光功率 150mW
- 取代原 `tools/lab/cni_laser_fake_driver`

## 依赖

- `odmr-types`
- `serialport`

## 参考

- `docs/equipment_manual/CNI Laser psu-sr/RS232语言协议_恒功率.md`
- `docs/equipment_manual/CNI Laser psu-sr/激光器使用说明书.md`

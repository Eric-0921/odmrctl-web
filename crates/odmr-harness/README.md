# odmr-harness

**Layer 3** — Mock/测试 harness。

## 职责

- `FakeDevice` 实现（mock SMB100A、mock OE1022D 等）
- Mock 设备注册到 DeviceManager
- 测试 fixture 构建工具
- 可配置的设备行为模拟（延迟、噪声、错误注入）

## 依赖

- `odmr-device`
- `odmr-types`

## 被

- 所有集成测试

## 参考

- `docs/prd/11_harness_mock_replay_prd_v0.2.md`

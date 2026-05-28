# odmr-replay

**Layer 3** — 离线数据回放服务。

## 职责

- 从 raw bin + index.jsonl 重建采集数据流
- 向 TraceService 推送回放数据
- 支持 GUI 回放控制（播放/暂停/跳转/倍速）

## 依赖

- `odmr-logging`
- `odmr-types`

## 参考

- `docs/prd/11_harness_mock_replay_prd_v0.2.md`

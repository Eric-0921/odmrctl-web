# odmr-replay

**Layer 3** — Canonical replay and legacy rawbin migration for ODMR run artifacts.

## 职责

- 从 canonical run 目录（`events.jsonl` + `index.jsonl` + `raw/*.rall`）重建采集数据流
- 支持三种回放模式：`OriginalTimestampPaced`、`ParseOnly`、`AsFastAsPossible`
- Legacy rawbin / manifest 目录兼容：自动适配为 canonical 回放会话
- `migrate_legacy_run_to_canonical()` — legacy 运行目录迁移到 canonical 格式
- `open_replay_session()` + `replay_trace()` — 标准回放 API

## 依赖

- `odmr-oe1022d` — RALL 帧解析
- `odmr-types`
- `serde`
- `serde_json`

## 参考

- `docs/prd/11_harness_mock_replay_prd_v0.2.md`

# odmr-logging

**Layer 3** — 数据落盘服务。

## 职责

- `RawRecorder`：raw binary 流写入
- `IndexWriter`：index.jsonl 写入
- `EventWriter`：events.jsonl / warnings.jsonl / errors.jsonl 写入
- Run directory 结构管理

## 依赖

- `odmr-types`

## 不负责

- Parquet/CSV 转换（Python 离线层负责）
- 数据可视化

## 参考

- `docs/prd/07_data_logging_file_format_prd_v0.2.md`

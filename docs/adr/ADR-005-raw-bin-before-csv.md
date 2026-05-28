# ADR-005: 实时采集先写 Raw Bin，实验后再导出 CSV

## Status

Accepted

## Date

2026-05-28

## Context

ODMR 自动化系统需要长时间、连续、可回放的数据采集链路。第一阶段的关键数据源是 OE1022D 锁相放大器的高速采集流，后续还会叠加 SMB100A、磁场轴、激光器、recipe step、设备状态与安全事件。

系统不能把 CSV 当成实时采集格式，原因如下：

1. **实时采集线程必须最小化工作量**：采集线程应优先完成读取、时间戳、写入与错误恢复，不能承担文本格式化、浮点转字符串、CSV quoting、列宽组织、复杂 flush 策略等工作。
2. **CSV 不适合作为唯一原始证据**：CSV 更适合给 Origin、Excel、Python notebook 或人工检查使用，但不应作为不可逆的原始记录。
3. **每个数据点必须能回溯**：数据必须能追溯到 recipe step、设备状态、命令事件、采集时间戳、原始 frame 偏移量和解析版本。
4. **采集和解析必须解耦**：采集阶段先保留原始二进制证据，实验后再解析成 Parquet / CSV，避免解析逻辑 bug 破坏原始数据。
5. **UI 不应接触原始二进制 frame**：GUI 只展示后端降采样结果和运行状态，不直接解析 RALL? 或其他 binary frame。

外部格式参考：CSV 是广泛交换格式，但不同应用之间存在细微差异，Python 官方文档也指出 CSV 缺少完全明确的统一标准；Parquet 是面向高效存储和分析的列式格式；Arrow IPC 文件格式支持固定数量 record batch 的随机访问，适合后处理和内存映射场景。

## Decision

第一阶段采用以下数据落盘策略：

```text
实时阶段：
  只写 raw bin / index.jsonl / events.jsonl / snapshots

实验后处理阶段：
  raw bin + index.jsonl -> parsed.parquet -> export.csv

长期可信源：
  raw bin + index.jsonl + events.jsonl + resolved_recipe.json + station_snapshot.json

人工交换格式：
  export.csv
```

也就是说：

```text
CSV 只是导出格式，不是实时采集格式，不是 canonical source of truth。
```

## Run Directory Layout

每次实验创建独立 run directory：

```text
runs/
  <run_id>/
    run_manifest.json
    station_snapshot.json
    resolved_recipe.json
    safety_report.json

    raw/
      oe1022d.raw.bin
      oe1022d.index.jsonl

    events/
      events.jsonl
      device_commands.jsonl
      warnings.jsonl

    parsed/
      oe1022d.parquet
      run_table.parquet

    export/
      oe1022d.csv
      summary.csv

    logs/
      executor.log
      parser.log
```

## Canonical Files

### `run_manifest.json`

记录一次 run 的元信息。

必须包含：

```json
{
  "run_id": "2026-05-28T12-00-00Z_odmr_001",
  "schema_version": "run-manifest.v1",
  "created_at_utc": "2026-05-28T12:00:00Z",
  "operator": "manual-or-agent-assisted",
  "project": "odor-ctl-web-PRD",
  "data_format_version": "raw-bin-index-v1",
  "status": "running | completed | aborted | failed"
}
```

### `station_snapshot.json`

记录实验开始时的 station 与 device registry 快照。

必须覆盖：

```text
device_id
transport
port / visa_address / socket_address
idn
capabilities
safety limits
firmware / driver version
connection status
```

### `resolved_recipe.json`

记录已经展开、校验、插入 dwell / settle / average 的最终执行 recipe。

Executor 只接受 `resolved_recipe.json`，不接受未展开的 recipe。

### `safety_report.json`

记录 safety checker 对 resolved recipe 的判定。

如果 `safety_report.status != "passed"`，Executor 不允许开始实验。

### `oe1022d.raw.bin`

实时采集的原始二进制文件。

第一阶段定义：

```text
append-only
只写入从 OE1022D 采集接口收到的原始 frame payload 或最小封装后的 frame record
不做浮点转换
不做 CSV 字符串化
不做 UI 降采样
不做复杂分析
```

推荐 record v1：

```text
magic:        4 bytes   "ODMR"
version:      u16
header_len:   u16
frame_seq:    u64
mono_ns:      u64
wall_ns_utc:  i128 or split seconds/nanos
payload_len:  u32
payload_crc:  u32
payload:      [u8; payload_len]
```

如果后续决定 raw bin 必须完全保存设备原始字节流，则 record header 可以移入 `index.jsonl`，但必须仍能做到 offset-level replay。

### `oe1022d.index.jsonl`

索引文件。一行对应一个 raw frame 或一个逻辑 frame record。

每行必须包含：

```json
{
  "frame_seq": 1024,
  "raw_file": "raw/oe1022d.raw.bin",
  "offset": 12345678,
  "nbytes": 512,
  "monotonic_ns": 987654321000,
  "wall_time_utc": "2026-05-28T12:00:03.123456Z",
  "recipe_step_id": "step.rf_sweep.00042",
  "device_id": "oe1022d.main",
  "parser_version": null,
  "crc32": "0x1234abcd"
}
```

作用：

```text
快速定位 raw frame
恢复中断 run
支持 replay
把数据点连接回 recipe step
把 raw 与 parsed/parquet/csv 建立可审计关系
```

### `events.jsonl`

事件日志。一行一个事件。

必须记录：

```text
run_started
run_finished
step_started
step_finished
device_command_sent
device_command_returned
device_warning
device_error
safety_rejection
manual_stop
emergency_stop
parser_error
export_finished
```

推荐字段：

```json
{
  "event_id": "evt_000001",
  "event_type": "device_command_sent",
  "monotonic_ns": 987654321000,
  "wall_time_utc": "2026-05-28T12:00:03.123456Z",
  "recipe_step_id": "step.rf_sweep.00042",
  "device_id": "smb100a.main",
  "message": "FREQ 2.882GHz",
  "severity": "info"
}
```

### `parsed.parquet`

实验后解析生成的结构化分析文件。

第一阶段建议按设备或逻辑表拆分：

```text
parsed/oe1022d.parquet
parsed/run_table.parquet
```

最低字段：

```text
run_id
frame_seq
sample_index
monotonic_ns
wall_time_utc
recipe_step_id
x
y
r
theta
raw_offset
raw_nbytes
parser_version
```

Parquet 是后续 Python / Polars / pandas / notebook 分析的优先格式。

### `export.csv`

实验后从 Parquet 或 parsed table 导出的人工交换格式。

限制：

```text
不作为实时写入格式
不作为唯一真相来源
不保证包含所有 raw provenance
不允许覆盖 raw bin / index / events
```

CSV 只用于：

```text
人工查看
Origin / Excel 导入
论文图表预处理
小规模数据交换
```

## Rejected Alternatives

### Alternative A: 实时直接写 CSV

Rejected.

原因：

```text
采集线程负担过重
文本格式化开销不可控
CSV dialect / quoting / locale / newline 容易产生兼容性差异
难以保留完整 raw frame
难以支持 replay
解析 bug 会直接污染唯一数据源
```

### Alternative B: 实时直接写 Parquet

Deferred.

原因：

```text
Parquet 适合分析和压缩存储，但实时写入需要 row group、schema、flush、partial file recovery 等策略
第一阶段不把采集线程绑定到复杂列式 writer
后处理阶段生成 Parquet 更简单、更可靠
```

后续如果采集链路稳定，可以评估：

```text
parser thread streaming parquet writer
periodic row group flush
crash-resumable parquet segment
```

### Alternative C: SQLite 作为实时采集主存储

Rejected for first implementation.

原因：

```text
SQLite 适合元数据、事件或低频结构化记录
不适合作为高频 raw frame 的第一落盘路径
数据库事务、锁和 schema migration 会增加实时路径复杂度
```

### Alternative D: 只保存 Parquet，不保存 raw bin

Rejected.

原因：

```text
无法证明 parser 没有丢失或误解原始 frame
无法重放旧数据验证新 parser
无法在后续发现 bug 后重新解析
不满足可追溯实验记录要求
```

## Consequences

### Positive

```text
实时采集路径更短
UI 卡顿不影响 raw 落盘
后处理 parser 可迭代
raw 数据可回放
每个 parsed sample 可追溯到 raw offset 与 recipe step
CSV 导出失败不影响实验原始记录
支持 mock / replay / regression test
```

### Negative

```text
文件数量增加
需要维护 raw bin parser 和 replay tool
实验结束后需要额外转换步骤
人工不能直接打开 raw bin
磁盘占用可能高于压缩格式
```

### Mitigation

```text
提供 raw replay tool
提供 parquet converter
提供 csv exporter
提供 run directory validator
提供 summary.json 方便快速检查
长期 run 可按大小或时间切分 raw segment
```

## Runtime Rules

### Acquisition Thread

允许：

```text
read frame
attach monotonic timestamp
attach wall timestamp
append raw bin
append index.jsonl
report minimal status
```

禁止：

```text
CSV 写入
复杂分析
图表绘制
GUI 调用
长时间锁
网络上传
AI 调用
```

### Parser Thread / Offline Parser

允许：

```text
读取 raw bin + index
解析 frame
生成 parquet
生成 summary
生成 csv export
校验 frame crc / length / sequence
```

禁止：

```text
修改 raw bin
修改原始 index offset
隐式覆盖旧 parsed 文件
在没有 parser_version 的情况下写 parsed result
```

### GUI

允许：

```text
显示运行状态
显示降采样曲线
显示最新 parsed preview
触发 export
打开 run directory
```

禁止：

```text
解析 raw frame
直接写 CSV
直接访问采集串口
绕过 Rust 后端读取 raw bin
```

## Error Handling

### Crash During Acquisition

恢复策略：

```text
扫描 index.jsonl 最后一条完整记录
检查 raw offset + nbytes 是否存在
如果 raw 末尾存在 partial frame，截断到最后完整 offset
标记 run_manifest.status = "aborted" 或 "recoverable"
记录 recovery event
```

### Parser Failure

处理策略：

```text
保留 raw bin 和 index
写 parser_error event
不得删除旧 parsed 文件
新 parsed 输出必须写入带 parser_version 的新目录或新文件
```

### Export CSV Failure

处理策略：

```text
不影响 run canonical data
记录 export error
允许重新从 parquet 导出
```

## Acceptance Criteria

第一阶段验收标准：

```text
连续采集 30 min，raw bin 持续增长且无 UI 阻塞
index.jsonl 每条记录 offset / nbytes 可定位 raw frame
events.jsonl 能复原 run step 顺序
raw replay tool 能重新生成 parsed parquet
parsed parquet 与旧 Python 解析结果一致
CSV 只能由 export command 生成，实时目录中不会出现持续写入的 CSV
随机中断后能恢复到最后完整 frame
每个 parsed sample 能追溯到 raw_offset、frame_seq、recipe_step_id
```

## Test Harness Requirements

必须提供：

```text
fake RALL frame generator
raw bin writer unit test
index consistency test
partial frame crash recovery test
raw replay test
parser parity test
parquet export test
csv export test
30 min soak test
UI load test: UI 卡顿不影响 acquisition writer
```

## Agent Constraints

AI agent 可以：

```text
生成 parser 测试样例
生成 schema 草案
解释 run directory
生成 CSV export 脚本草案
检查 data logging PRD 与 ADR 一致性
```

AI agent 不可以：

```text
修改 raw bin
伪造 raw bin
在 live run 中直接写采集文件
绕过 executor 触发设备采集
把 CSV 改回实时采集主格式
删除 index / events / snapshots
```

## Migration Plan

### Phase 1

```text
实现 oe1022d.raw.bin
实现 oe1022d.index.jsonl
实现 events.jsonl
实现 raw replay tool
实现 parsed.parquet 离线转换
实现 export.csv 离线导出
```

### Phase 2

```text
增加 raw segment 切分
增加 checksum manifest
增加 parser version registry
增加 run validator
增加 parquet schema evolution
```

### Phase 3

```text
评估 streaming parquet writer
评估压缩 raw bin
评估跨 run 数据目录索引
评估长期数据归档策略
```

## Review Triggers

以下情况出现时，需要重新评估本 ADR：

```text
OE1022D frame rate 明显低于预期，不再需要 raw-first 设计
raw bin 磁盘占用成为主要瓶颈
需要实时跨进程分析完整数据流
需要与第三方软件实时共享表格数据
Parquet streaming writer 已经稳定且 crash recovery 经过验证
```

## References

- Apache Parquet Documentation, Apache Parquet is an open source column-oriented data file format designed for efficient data storage and retrieval.
- Python `csv` module documentation, CSV lacks a fully well-defined standard and different applications may produce subtle differences.
- Apache Arrow IPC documentation, file / random access format supports random access and is useful with memory maps.
- Project PRD: `07 Data Logging & File Format Sub-PRD.md`.
- Project PRD: `03 OE1022D Acquisition Sub-PRD.md`.
- Project PRD: `11 Harness - Mock - Replay Test Sub-PRD.md`.

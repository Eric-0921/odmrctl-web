# odmr-dataset-acquisition

OE1022D 1 kS/s 数据集采集程序 (sibling worktree of `odmrctl-web`)

> **目的**:用 1 kS/s 时间分辨率采集 OE1022D 锁相放大器的 B 通道测量值,在不同
> (OE1022D 配置 / SMB100A 微波参数 / 三轴磁场矢量 / 激光功率) 配置下,长期
> 累积大量谱线,作为后续 ML 模型的训练数据。

## 与主仓的关系

- 主仓路径:`../odmrctl-web/`
- 主仓分支 `main` 镜像至此 worktree(分支 `feat/oe1022d-dataset`)
- 通过 `path` 依赖引用主仓的 `odmr-types` / `odmr-device` / `odmr-oe1022d`,
  不复制代码,合回时只合本仓新增的 4 个 crate 即可
- 其他 3 层(SMB100A / Magnetic / Laser)在 v0.1 只记配置,不出现在 transport

## 4 个 crate

| Crate | 职责 | 计划 commit |
|-------|------|------------|
| `oe1022d-transport` | 串口枚举、IDN? 探测、RALL? 固定长度读取、循环控制 | C3 + C4 |
| `oe1022d-acquisition` | 4 线程核心 (acq/parser/writer/downsample) + ndjson 落盘 | C5 + C6 + C7 |
| `oe1022d-config-stack` | 4 层 onion profile 加载 + 校验 | C8 |
| `oe1022d-chart-core` | Tauri commands + 给前端的降采样窗口 | C9 |

前端 (Tauri + React + Plotly) 在 `apps/desktop/`,留待 M2 (C10) 引入。

## 立即跑通

```sh
cd /Users/erictseng/Documents/codex_git/odmr-dataset-acquisition
cargo build      # 0 错
cargo test       # 4 个 crate 各 1 个 sanity test
```

## 路线图

- **C1 (本次)**:scaffold — 4 个 crate 骨架 + workspace + path dep
- **C2**:todo
- **C3**:serial port enumeration + IDN? probe,K1-K8 坑显式处理
- **C4**:RALL? fixed-length reader + 连续循环
- **C5**:AcquisitionThread OS 线程绑核,RALL? cycle 实测 ~800ms/帧
- **C6**:ParserThread 50-sample split + 1ms 间隔时间戳
- **C7**:WriterThread 批量 flush + events.jsonl
- **C8**:4 层 onion profile 加载
- **C9**:Tauri commands + 30Hz downsample
- **C10**:Plotly AcquirePage
- **C11**:真机 5 min 跑通 (Mac, OE1022D)

详见 `docs/PRD.md`(计划在 C2 落地)。

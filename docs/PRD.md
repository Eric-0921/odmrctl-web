# odmr-dataset-acquisition PRD v0.1

> 项目代号: `odmr-dataset-acquisition`
> 工作树: `~/Documents/codex_git/odmr-dataset-acquisition/`(主仓 `odmrctl-web` 的 sibling worktree,分支 `feat/oe1022d-dataset`)
> 上一级 PRD: `../odmrctl-web/docs/prd/00_main_prd_v0.2.md`、`../odmrctl-web/docs/prd/03_oe1022d_acquisition_prd_v0.2.md`、`../odmrctl-web/docs/prd/08_gui_tauri_chart_prd_v0.3.md`
> 设备手册权威源: `../odmrctl-web/docs/equipment_manual/oe1022d/`(任何改动前先翻)
> 日期: 2026-06-05
> 状态: Design Draft, 待 owner 最终签字

---

## 1. 目的 (Purpose)

为 **ML 数据集采集** 而非 ODMR 实验自动化。本项目以 1 kS/s 时间分辨率采集 OE1022D 锁相放大器的 B 通道测量值,在不同 (OE1022D 配置 / SMB100A 微波参数 / 三轴磁场矢量 / 激光功率) 配置下长期累积大量谱线,作为后续 ML 模型的训练数据。

### 1.1 物理事实(按设备手册 5.2.11 + M2.5 实测)

| 维度 | 值 | 来源 |
|------|----|----|
| OE1022D A/D 采样率 | 312.5 kHz(双通道独立 ADC) | 手册 §2 `02_fundamentals.md` |
| RALL? 帧内 50 个点时间间隔 | **1 ms**(1 kS/s) | 手册 §5.2.11 `05_oe1022d_rall_global_data_config_reading.md:10` |
| RALL? 帧刷新率 | 20 Hz(50ms 数据) | 同上 |
| RALL? 帧大小 | **固定 12288 bytes** 二进制,无 terminator | 同上 |
| 波特率 | **921600**(USB CDC 不可协商) | 手册 §2.2a + 实测 |
| 软件读帧耗时(真机 D6130220) | **~900ms/帧** | C5.5 真机抓的 |
| → 1kHz 时间序列真实有效数据率 | **~62.5 sample/s** | = 50 sample × 1.25 帧/s |

> **"完全连续" 定义**:软件层每 ~900ms 抓到 1 帧,从不丢失、从不重复;帧内 50 个点保持 1ms 间隔的相对时间戳;不允许 UI 暂停、写盘阻塞、IPC 卡顿导致帧间隔 > 1s。

### 1.2 与主仓的关系

| 允许引用 | 禁止引用 |
|----------|----------|
| `odmr-oe1022d`(commands / parser / fake) | `odmr-executor` / `odmr-recipe` / `odmr-compiler` |
| `odmr-types` / `odmr-device` | `odmr-safety` / `odmr-smb100a` / `odmr-maynuo-m8812` / `odmr-mag` |
| 设备手册 `docs/equipment_manual/oe1022d/` | 其他手测报告不需要 |

> 合回时只合本仓新增的 4 个 crate。

---

## 2. 用户角色

| 角色 | 主要诉求 | 频率 |
|------|----------|------|
| 数据集操作员 | 选 onion recipe → 跑采集 → 看实时谱线 → 1 键停止 → ndjson 落盘 | 每天多次 |
| 实验物理研究员 | 不同参数配置(磁场 / smb 频率 / 功率)采一组 | 每周多次 |
| ML 工程师 | ndjson 拉到训练环境;关心字段齐 + 时间戳真 + 配置元数据全 | 离线 |

---

## 3. 4 层环配置模型(洋葱)

```
Layer 4: 激光 (CNI Laser)         暂不到
Layer 3: 三轴磁场 (Maynuo M8812 ×3)  独立可关
Layer 2: 微波源 (SMB100A)         独立可关
Layer 1: 采集 (OE1022D)          必到
Layer 0: 标签 (4 层配置快照)     自动
```

- **每层 ON/OFF 独立**;关闭某层时,采集仍能跑(只是 dataset 缺一维标签)
- **本项目 v0.1 只接 OE1022D 真机**,其他 3 层只记配置,不接 transport
- **每层一个 profile JSON**(参照主仓 `examples/device_profiles/*.json` 命名)

---

## 4. ML 数据集定义

### 4.1 抽象层

| 抽象 | 含义 | ML 用途 |
|------|------|---------|
| **Sample** | 1 个 (时间戳, B-X, smb 频率) | 训练样本 |
| **Frame** | RALL? 一帧(50 sample @ 1kHz) | 数据块 |
| **Sweep** | smb 频率扫描一次(物理实验) | 一条谱线 |
| **Run** | 一次完整采集(从 Start 到 Stop) | 1 个 ndjson 目录 |

### 4.2 样本 = (时间, B-X, 微波频率) — 1 sample = 1 ndjson line

```jsonc
{
  "t_mono_ns": 1780206577446000000,
  "t_wall_ms": 1780206577446,
  "frame_sequence_no": 7,
  "sample_in_frame": 23,
  "oe1022d_B_X": -4.96e-3,
  "oe1022d_B_Y": 2.36e-3,
  "oe1022d_B_R": null,
  "oe1022d_B_freq_hz": 0.554,
  "oe1022d_overload": false,
  "oe1022d_pll_locked": true,
  "smb100a_freq_hz": 2.882e9,
  "smb100a_power_dbm": -10.0,
  "magnetic_B_x_nt": 0.0,
  "magnetic_B_y_nt": 0.0,
  "magnetic_B_z_nt": 1000.0,
  "laser_power_mw": 50.0,
  "laser_state": "ON"
}
```

### 4.3 落盘结构(1 run = 1 目录)

```
runs/
  2026-06-05_demo_001/
    metadata.json           # 4 层 profile + 启动时间 + 操作员
    samples.ndjson          # 1 sample = 1 line
    events.jsonl            # start/stop/overload/pll_lost/frame_lost
    oe1022d_config_snapshot.json  # 采集结束时 OE1022D 当前配置区快照
```

---

## 5. 采集核心(4 线程)

```
[OS Thread 1: Acquisition]      port.read() loop → 12288 bytes
                                → push RawFrameEnvelope
[OS Thread 2: Parser]           pop frame → 50 samples × 1ms timestamps
                                ├→ ParsedRing (UI 30Hz emit)
                                └→ WriterQueue → WriterThread
[OS Thread 3: Writer]           pop WriterQueue → batch flush → ndjson
[Tauri event loop]              30Hz 读 ParsedRing → emit
```

### 5.1 抗延迟设计

| 问题 | 设计对策 |
|------|----------|
| Acquisition 同步 read 被 UI/写盘拖慢 | OS 线程绑核(可选),只用 `port.read()`,不做解析/写盘 |
| macOS 每次 read 只回 ~1020B | 4KB buffer + while loop,凑齐 12288 或 900ms 视为失败 |
| 设备 800ms 没准备好 | 只发一次 RALL?,超时降级,记 `frame_short` 事件,绝不悄悄补 |
| 写盘 fsync 阻塞 parser | WriterThread 独立 OS 线程,8KB 缓冲批量写 |

### 5.2 K1 残留(真机实测)

C5.5 真机跑出来:**前 1-2 帧 13260 / 12476 bytes(包含 IDN? 响应尾巴),后稳 12288**。C6 parser:
- frames < 12288 → `FrameTooShort`,跳过
- frames > 12288 → 截断到 12288,丢弃尾部,标记 `partial_warmup = true`

---

## 6. 端口扫描 + 身份识别

### 6.1 流程

1. `serialport::available_ports()` 列出所有端口
2. 逐个打开 921600/8N1/no-flow-control
3. `clear(Input)` + 发 `*IDN?\r` + 读直到 CR/LF
4. 解析 IDN(兼容两种固件格式)
5. 匹配 `manufacturer == "SSI"` && `model.contains("OE1022D")` → 候选

### 6.2 指纹持久化

`~/.oe1022d_chart/devices.toml` 存 SN + 上次 port + 上次时间。每次启动全扫,逐个 IDN? → 拿 SN → 查表。

### 6.3 IDN? 格式兼容(实测)

- **老固件** `D6522078 Ver6.3200831`: `SSI,LIA-OE1022D,D6522078,Ver6.3200831`(4 字段)
- **新固件** `D6130220 Ver6.32111110`: `SSI LIA-OE1022D,SN:D6130220,Version:Ver6.32111110`(3 字段带标签)
- **身份 buffer 固定长度,前导 NUL 填充** → `trim_start_matches(|c| c=='\0' || c.is_whitespace())`

---

## 7. 前端(Tauri v2 + React + Plotly)

- Plotly 而非 Recharts(性能 + PNG 导出)
- X 轴单向(单调时间)
- X 轴长度用户输入(500/1000/5000),超过 reset
- 30Hz emit(不接触 raw 全量,只看 DownsampleService 输出)

---

## 8. 验收标准

### v0.1 (本周)

- [x] C1: 4 crate scaffold + path dep
- [x] C3: 端口枚举 + IDN 解析 + K1-K8
- [x] C4: RALL? 固定 12288 读取 + 连续循环
- [x] C5: SerialRallLink + 绑核 OS 线程
- [x] C5.5: 自动发现 OE1022D + 真机 5 帧
- [x] C6: ParserThread + 1ms 间隔时间戳 + K1 warmup
- [ ] C7: WriterThread 落盘 + ndjson
- [ ] C8: 4 层 onion profile 加载
- [ ] C9: Tauri commands + 30Hz downsample
- [ ] C10: Plotly AcquirePage
- [ ] C11: 真机 5 min 端到端

### 性能验收

- [ ] mock 30 min 稳定运行
- [ ] 真机 5 min 跑通
- [ ] UI 30Hz 刷新不掉帧
- [ ] raw writer 无 silent failure
- [ ] frame interval < 1s(实测 ~900ms,符合 M2.5 范围)

---

## 9. 非目标

- 控制 SMB / Laser / 磁场(只记配置)
- 实时 CSV 写盘
- 在前端解析 raw 12288 bytes
- 实时拟合 / FFT / 寻峰
- 任何 pulse 模式 / Rabi / T1 / T2
- AI agent 自动控制
- 远程实验服务
- 温度层

---

## 10. 风险

| 风险 | 缓解 |
|------|------|
| Mac 驱动抖动导致帧间隔 > 1s | 接受 800-1000ms,绝不悄悄补帧 |
| Plotly 30Hz 重绘性能不够 | 降频到 10Hz;切 uPlot |
| USB 拔插后 devices.toml 失效 | 启动全扫,SN 匹配,提示"新位置" |
| OE1022D 真机长时间占用 | 进程内 Mutex<()>(),不允许并发 |
| ndjson 文件过大 | 1kHz × 200B × 1h ≈ 700MB,按 run 分目录 |

---

## 11. 仓库结构

```
odmr-dataset-acquisition/
├── Cargo.toml                  # 4-crate workspace + path deps
├── apps/desktop/               # Tauri v2 + React + Plotly (C10)
├── crates/
│   ├── oe1022d-transport/      # C3-C6 ✅
│   ├── oe1022d-acquisition/    # C5+C7
│   ├── oe1022d-config-stack/   # C8
│   └── oe1022d-chart-core/     # C9
├── docs/PRD.md                 # 本文
└── README.md
```

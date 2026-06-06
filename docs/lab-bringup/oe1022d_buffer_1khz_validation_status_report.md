# OE1022D Buffer 子系统 1 kHz 采样验证 — 当前状态报告

**日期**: 2026-06-06
**状态**: 文档审计完成，真机验证尚未启动
**报告人**: Claude Code Agent

---

## 一、总体进展

| 阶段 | 状态 | 说明 |
|---|---|---|
| 文档审计 | **已完成** | 四份关键文档已通读并提炼 |
| 代码准备 | **已完成** | Buffer 命令 builder + probe 工具已存在 |
| 标准化 preflight | **未执行** | 尚未创建最小 station profile 并运行 `common_preflight` |
| 真机命令验证 | **未执行** | 尚未对 SRATD/SLEND/SSLED/STRGD/SPRMD 做 set/query 一致性验证 |
| 采样点数增长验证 | **未执行** | 尚未启动 STRDD 并周期性查询 SPTSD? |
| TRCAD? 读取验证 | **未执行** | 尚未确认 ASCII 浮点串返回格式和解析可行性 |
| 最终结论 | **未形成** | 无法回答"Buffer 路到底卡在哪一步" |

---

## 二、文档审计结论

### 2.1 `RALL?` 的文档语义

| 属性 | 文档结论 |
|---|---|
| 接口限制 | **USB-only**，RS232 不支持该指令 |
| 返回总长度 | 固定 **12288 bytes** 二进制帧 |
| 每帧点数 | **50 个点**（每参数 50 个，共 20 个参数） |
| 点间隔 | **1 ms** |
| 刷新周期 | **每 50 ms 更新一次** |
| 本质语义 | **窗口快照**，不是连续流。返回的是"之前 50 ms 的测量数据"，设备侧刷新率为 20 Hz |
| 是否适合 1 kHz 连续采集 | **不适合**。单帧内部确实有 1 ms 间隔的点，但帧与帧之间是 50 ms 的快照，无法覆盖中间 49 ms |

> 关键区分：`RALL?` 帧内 1 ms 点间隔 ≠ `RALL?` 是 1 kHz 连续采样接口。前者是"50 ms 窗口内的 50 个历史点"，后者需要可持续的、高覆盖率的 streaming 语义。

### 2.2 Buffer/SAMPLE 子系统的文档语义

| 命令 | 文档语义 |
|---|---|
| `SRATD i,x` | 设置采样步进时间。范围 **1 ms ~ 100 s**，最小分辨率 **1 ms** |
| `SLEND i,j` | 设置采样长度。最大 **16384** 点 |
| `SSLED i,j,k` | 绑定 Buffer j (1~4) 到参数 k (0~21) |
| `STRGD i,j` | 触发方式：0=INT 内部，1=EXT 外部 |
| `SPRMD i,j` | 运行模式：0=Single 单次，1=Loop 循环 |
| `STRDD i` | 开始/继续采样 |
| `PAUSD i` | 暂停采样 |
| `RESTD i` | 重置 Buffer（清空 Buffer1~4） |
| `SPTSD? i` | 查询已存储点数。Buffer 重置后返回 0 |
| `TRCAD? i,j,k,l` | 从 Buffer j 的第 k 个点开始读取 l 个点。返回 **ASCII 浮点数**，逗号分隔。`k+l ≤ 16384` |

**文档是否明确支持 1 kHz**: **是**。手册明确写明 `SRATD` 最小分辨率为 1 ms，即理论上可配置为 1 kHz 采样率。

**但文档结论 ≠ 真机结论**。以下问题必须通过真实设备验证：

1. `SRATD=0.001`（1 ms）是否被设备 firmware 真正接受，还是会被拒绝或量化到更粗粒度？
2. `SPTSD?` 返回的"已存储点数"是否严格按 1 ms 步进增长？
3. `TRCAD?` 的 ASCII 浮点串在大量数据（如 1024/4096 点）下的返回延迟和稳定性如何？
4. Single 模式是否停在 `SLEND` 上限？Loop 模式是否确实循环覆盖？

---

## 三、代码现状

### 3.1 已存在的代码

| 文件 | 内容 | 状态 |
|---|---|---|
| `crates/odmr-oe1022d/src/commands.rs:298-383` | Buffer 子系统全部命令 builder（SRATD/SLEND/SSLED/STRGD/SPRMD/STRDD/PAUSD/RESTD/SPTSD?/TRCAD?） | ✅ 已存在 |
| `crates/odmr-oe1022d/src/lib.rs:135-187` | Buffer 命令的 golden unit tests | ✅ 已存在 |
| `tools/lab/oe1022d_buffer_probe/src/main.rs` | 直接串口操作的 probe 工具，可配置 step/length/mode、周期性轮询 SPTSD?、读取 TRCAD? | ✅ 已存在 |
| `tools/lab/common_preflight/` | 标准化 preflight 工具 | ✅ 已存在 |
| `crates/odmr-preflight/README.md` | preflight 架构说明 | ✅ 已存在 |

### 3.2 代码缺口

| 缺口 | 说明 | 影响 |
|---|---|---|
| `oe1022d_buffer_probe` **未走 preflight** | 当前 probe 直接 `serialport::new().open()`，绕过了 `common_preflight --preflight-only` | 违反仓库标准化流程，无法产生 station ledger 和 preflight artifact |
| `fake.rs` 无 Buffer 状态模拟 | 假设备不模拟 sample_time / sample_length / buffer_selector / trace_data | 无法做 mock-first 回归测试 |
| `odmr-types` 无 Buffer 数据类型 | 缺少 `BufferSample` / `TraceChunk` / `OeAcquisitionMode` | executor 和 logging 无法接入 |

---

## 四、真机验证待办清单

### 4.1 第二阶段：标准化连接

- [ ] 创建只含 OE1022D 的最小 `station.json`
- [ ] 运行 `cargo run -- --station-profile <profile> --out-dir <dir> --preflight-only`
- [ ] 确认 preflight 成功并保存 artifact 路径

### 4.2 第三阶段 A：命令 set/query 一致性

- [ ] `SRATD 2,0.001` → `SRATD? 2` → 回读是否一致
- [ ] `SLEND 2,128` → `SLEND? 2` → 回读是否一致
- [ ] `SSLED 2,1,1` → `SSLED? 2,1` → 回读是否一致
- [ ] `STRGD 2,0` → `STRGD? 2` → 回读是否一致
- [ ] `SPRMD 2,0` → `SPRMD? 2` → 回读是否一致

### 4.3 第三阶段 B：采样点数增长

- [ ] 配置 `SRATD=1ms, SLEND=128, SPRMD=single`
- [ ] `RESTD 2` → `STRDD 2` → 周期性 `SPTSD? 2`
- [ ] 记录点数是否随时间增长、增长斜率是否 ≈ 1000 点/s
- [ ] 验证 single 模式是否停在 128
- [ ] 切换 `SPRMD=loop`，验证是否出现循环覆盖行为

### 4.4 第三阶段 C：TRCAD? 读取

- [ ] 在确认有稳定点数后，发送 `TRCAD? 2,1,0,128`
- [ ] 确认返回格式：ASCII 浮点、逗号分隔
- [ ] 确认返回点数是否严格等于请求长度
- [ ] 确认数据可解析为有效数值数组

---

## 五、关键文档与代码路径

| 类型 | 路径 |
|---|---|
| 决策文档 | `/Users/erictseng/Documents/codex_git/odmrctl-web/docs/decisions/oe1022d-high-speed-buffer-acquisition.md` |
| RALL? 手册 | `/Users/erictseng/Documents/codex_git/odmrctl-web/docs/equipment_manual/oe1022d/05_oe1022d_rall_global_data_config_reading.md` |
| 命令手册 | `/Users/erictseng/Documents/codex_git/odmrctl-web/docs/equipment_manual/oe1022d/05_oe1022d_remote_programming_commands_55_74.md` |
| Buffer 命令 builder | `/Users/erictseng/Documents/codex_git/odmrctl-web/crates/odmr-oe1022d/src/commands.rs:298-383` |
| 现有 probe 工具 | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/oe1022d_buffer_probe/src/main.rs` |
| preflight 工具 | `/Users/erictseng/Documents/codex_git/odmrctl-web/tools/lab/common_preflight/` |
| 本报告 | `/Users/erictseng/Documents/codex_git/odmrctl-web/docs/lab-bringup/oe1022d_buffer_1khz_validation_status_report.md` |

---

## 六、下一步行动建议

1. **立即执行 preflight**：创建最小 station profile，运行 `common_preflight --preflight-only`，确认设备可达。
2. **补 preflight 合规性**：当前 `oe1022d_buffer_probe` 直接开串口，违反仓库流程。要么改造 probe 接入 preflight 后的 DeviceLock，要么在 preflight 通过后另写最小验证脚本。
3. **不要并行占用设备**：OE1022D 是共享硬件资源，确保无其他进程（如 GUI）同时持有串口。
4. **保持最小修改**：只添加与 Buffer 验证直接相关的代码，不扩展 GUI、executor、logging。

---

## 七、诚实声明

**截至本报告生成时刻（2026-06-06），真机验证尚未执行。**

所有关于"Buffer 子系统是否支持 1 kHz"的结论，目前均来自手册文档推导。以下关键问题**尚无真机证据**：

- `SRATD=1ms` 是否被 firmware 真正接受
- `SPTSD?` 在 1 ms 步进下是否确实表现为 1000 点/s 增长
- `TRCAD?` 读取 1024+ 点时的延迟和稳定性
- Single/Loop 模式的边界行为

**在真机验证完成前，不能将"文档支持 1 ms"等同于"已确认 1 kHz 连续采样"。**

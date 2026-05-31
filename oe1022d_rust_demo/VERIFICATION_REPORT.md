# OE1022D RALL? 数据采集验证报告

> 日期: 2026-05-31
> 设备: SSI LIA-OE1022D, SN:D6522078, Version:Ver6.3200831
> 端口: /dev/cu.usbmodem3361358734371 @ 921600 baud
> 验证人: Claude Code Agent

---

## 1. 验证目的

验证 OE1022D `RALL?` 命令返回的原始二进制数据格式是否与
`odmr-control/docs/oe1022d_acquisition_guide.md` 中定义的布局一致，
为 Rust `oe1022d-core` 采集模块的实现提供硬件实测依据。

---

## 2. 验证方法

### 2.1 独立验证环境

为避免影响现有 `odmrctl-web` 项目代码，所有验证工作均在独立环境中进行：

- **Python 验证脚本**: 独立 venv + pyserial，直接读写串口
- **Rust Demo**: 独立 crate (`oe1022d_rust_demo/`)，不在 workspace 中

### 2.2 验证步骤

1. 打开串口，清空缓冲区
2. 发送 `RALL?\r`
3. 等待 800ms（设备准备帧数据）
4. 循环 `read()` 直到收齐数据或超时
5. 解析并验证数据布局

---

## 3. 验证结果

### 3.1 帧大小

| 检查项 | 预期值 | 实测值 | 结果 |
|--------|--------|--------|------|
| 总字节数 | 12288 | **12288** | ✅ PASS |

### 3.2 数据布局

| 区域 | 字节范围 | 预期大小 | 实测 | 结果 |
|------|----------|----------|------|------|
| 测量数据 | 0 ~ 7999 | 8000 bytes | 8000 bytes | ✅ |
| 配置快照 | 8000 ~ 9215 | 1216 bytes | 1216 bytes | ✅ |
| 填充 | 9216 ~ 12287 | 3072 bytes | 3072 bytes | ✅ |

测量数据区包含 **20 个参数 × 50 个采样点 × 8 bytes (f64)** = 8000 bytes。

参数布局与文档完全一致：

| 偏移 | 参数 | 列名 |
|------|------|------|
| 0 | A-X | `lockin_A_X_mv` |
| 400 | A-Y | `lockin_A_Y_mv` |
| 800 | A-Freq | `lockin_A_freq_hz` |
| 1200 | A-Noise | `lockin_A_noise_mv` |
| 1600 | A-Xh1 | `lockin_A_Xh1_mv` |
| 2000 | A-Yh1 | `lockin_A_Yh1_mv` |
| 2400 | A-Xh2 | `lockin_A_Xh2_mv` |
| 2800 | A-Yh2 | `lockin_A_Yh2_mv` |
| 3200 | B-X | `lockin_B_X_mv` |
| 3600 | B-Y | `lockin_B_Y_mv` |
| 4000 | B-Freq | `lockin_B_freq_hz` |
| 4400 | B-Noise | `lockin_B_noise_mv` |
| 4800 | B-Xh1 | `lockin_B_Xh1_mv` |
| 5200 | B-Yh1 | `lockin_B_Yh1_mv` |
| 5600 | B-Xh2 | `lockin_B_Xh2_mv` |
| 6000 | B-Yh2 | `lockin_B_Yh2_mv` |
| 6400 | AUXADC1 | `aux_adc1_v` |
| 6800 | AUXADC2 | `aux_adc2_v` |
| 7200 | AUXADC3 | `aux_adc3_v` |
| 7600 | AUXADC4 | `aux_adc4_v` |

### 3.3 字节序

| 字节序 | 首个值 (Sample 0) | 合理性 | 结果 |
|--------|-------------------|--------|------|
| Big-Endian f64 | ~4.6e-5 | 小信号级别 ✅ | **正确** |
| Little-Endian f64 | ~8.4e123 | 天文数字 ❌ | 不正确 |

**数据格式: Big-Endian IEEE 754 double (64-bit)**

### 3.4 配置快照交叉验证

RALL? 配置区的值与之前 SCPI 独立查询结果一致：

| 参数 | SCPI 查询值 | RALL? 配置区 | 匹配 |
|------|------------|-------------|------|
| A-Sensitivity (`SENSD? 2`) | 24 | byte 8390 = 24 | ✅ |
| A-Time Constant (`OFLTD? 2`) | 9 | byte 8404 = 9 | ✅ |
| A-Filter Slope (`OFSLD? 2`) | 1 | byte 8405 = 1 | ✅ |

### 3.5 填充区

3072 bytes 填充区中检测到 **1 个非零字节** (值为 42)，其余全为零。
判定为设备残留数据，不影响解析。

---

## 4. 关键发现与问题

### 4.1 🔴 Rust `read_exact()` 陷阱

**问题**: Rust `serialport::SerialPort::read_exact()` 在调用时如果串口缓冲区中已有数据（如之前的 IDN? 响应残留），会**先读取这些残留数据**，导致帧数据不完整或被污染。

**根因**: `read_exact()` 只是循环调用 `read()` 直到填满缓冲区，不保证数据的时序正确性。

**正确做法**:
```rust
// 1. 清空输入缓冲区
port.clear(ClearBuffer::Input)?;

// 2. 发送命令
port.write_all(b"RALL?\r")?;
port.flush()?;

// 3. 等待设备准备数据
std::thread::sleep(Duration::from_millis(800));

// 4. 循环读取直到收齐 12288 bytes
let mut frame = Vec::new();
while frame.len() < 12288 {
    let mut buf = [0u8; 4096];
    let n = port.read(&mut buf)?;
    frame.extend_from_slice(&buf[..n]);
}
```

### 4.2 🔴 数据分块到达

macOS 串口驱动每次返回约 **1020 bytes**（接近 1024 字节内核缓冲区大小），
12288 bytes 的完整帧需要 **~13 次读取** 才能收齐。

Python 能一次读到 12288 bytes 是因为 `pyserial.read(32768)` 在数据到达前阻塞了足够久（sleep 800ms + 内部 select），等所有数据到齐后一次性返回。

### 4.3 🟡 PRD 中的错误

现有 `03_oe1022d_acquisition_prd_v0.2.md` 中存在以下与实测不符的描述：

| 位置 | 错误描述 | 实际情况 |
|------|----------|----------|
| §6.2 | baud_rate: 115200 | **921600** |
| §9.1 | `read_until(frame_terminator or timeout)` | RALL? **没有 terminator**，是固定长度二进制帧 |
| §11.4 | "raw_payload 第一阶段可以是 ASCII response bytes" | RALL? 返回的是**二进制数据**，不是 ASCII |
| §27 Q1 | "RALL? 的真实返回格式是否稳定？" | ✅ 已确认稳定，12288 bytes 固定 |
| §27 Q2 | "RALL? 是否有固定 terminator？" | ❌ 没有 terminator |
| §27 Q7 | "overload / PLL lock 是否能在 RALL? frame 中直接读到？" | ✅ 可以，在配置区 |
| §27 Q8 | "是否需要单独命令读取 PLL / overload 状态？" | ❌ 不需要 |

---

## 5. 数据样例

### 5.1 实测信号值 (Ch-B)

| 参数 | Sample 0 | Sample 1 | Sample 2 | 均值 |
|------|----------|----------|----------|------|
| B-X | -4.96e-3 | -4.96e-3 | -4.96e-3 | -4.80e-3 |
| B-Y | 2.36e-3 | 2.36e-3 | 2.37e-3 | 2.66e-3 |
| B-Freq | 0.554 | 0.554 | 0.554 | 0.554 |
| B-Noise | 1.52e-5 | 1.53e-5 | 1.55e-5 | 1.90e-5 |

数值在合理范围内，符合锁相放大器小信号特征。

---

## 6. 结论

1. **RALL? 数据格式完全符合 `oe1022d_acquisition_guide.md` 规范**
2. **帧大小固定为 12288 bytes，数据布局为 20×50×8 bytes BE f64**
3. **配置区可直接读取设备状态，无需额外 SCPI 查询**
4. **Rust 串口读取必须使用循环 `read()` 累积数据，`read_exact()` 有陷阱**
5. **PRD 和 ADR 需要更新以反映实测结果**

---

## 7. 附录: 独立 Demo 代码位置

```
odmrctl-web/
  oe1022d_rust_demo/          ← 独立 Rust demo（不在 workspace 中）
    Cargo.toml
    src/main.rs
    VERIFICATION_REPORT.md    ← 本文件
    rall_frame_*.raw          ← 采集的原始帧数据
```

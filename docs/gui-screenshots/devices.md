# Devices

## Route
`/devices`

## Screenshot
![Devices](../../screenshots/devices.png)

## Visual Description

页面展示 **6 台实验设备的静态卡片**，采用 2 列网格布局：

1. **SMB100A** — RF / microwave signal generator
2. **OE1022D** — DSP lock-in amplifier / acquisition source
3. **Laser Controller** — laser controller placeholder（optional）
4. **Mag X (MAYNUO M8812)** — X-axis current source
5. **Mag Y (MAYNUO M8812)** — Y-axis current source
6. **Mag Z (MAYNUO M8812)** — Z-axis current source

每张卡片显示：设备名称、Role、Required by recipe（yes/no）、Connection status（均显示 "unavailable in GUI-M0"）、Mock status（static snapshot only）、Last known state。

底部有每个设备的禁用操作按钮组：
- SMB100A: Connect / Probe / Configure / Output ON / MOD ON
- OE1022D: Connect / Probe / Configure
- Laser: Connect / Emission ON
- Mag X/Y/Z: Connect / Set current / Output ON

所有按钮均为 `disabled`，原因标签包括 "M2 bring-up only"、"Forbidden in GUI-M0"、"Mock viewer only"。

页面顶部有一条蓝色 info banner：`No serial / USB / VISA / TCP socket probing exists in GUI-M0`。

## Code Structure

- **文件**: `src/routes/DevicesPage.tsx` (154 行)
- **数据**: 两个硬编码数组 — `devices[]`（6 台设备元数据）和 `disabledControls`（每设备禁用按钮映射）
- **组件**: 纯 CSS-in-JS，无外部 UI 库
- **交互**: 无 Tauri 命令；所有按钮 `disabled` + `cursor: not-allowed`
- **布局**: CSS Grid `repeat(2, 1fr)` 卡片网格，卡片内 Flexbox 垂直排列按钮组

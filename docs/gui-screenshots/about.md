# About / Boundaries

## Route
`/about`

## Screenshot
![About / Boundaries](../../screenshots/about.png)

## Visual Description

页面展示 **GUI-M0 的能力边界声明**，纯静态文本，分为三个区域：

1. **Boundary Statement** (全宽卡片):
   - "This GUI is mock-only."
   - "It does not connect to devices."
   - "It does not call executor."
   - "It does not send SCPI."
   - "It does not read OE1022D RALL?."
   - "It does not write experiment data."
   - "Future M2 integration must go through backend APIs, executor, and safety interlock."

2. **Allowed in M0** (左半卡片，绿色标题):
   - Display mock run summary
   - Display dry-run plan
   - Display safety report
   - Display events
   - Display artifact inventory
   - Display disabled future controls

3. **Forbidden in M0** (右半卡片，红色标题):
   - serial / USB / VISA / TCP socket access
   - SCPI sending
   - executor calls
   - hardware polling
   - raw data parsing
   - run data writing
   - AI live hardware control

4. **Future M1 / M2 Integration Path** (全宽卡片):
   - M1: read-only backend APIs (mock listing、static file loading、replay timeline、chart preview)
   - M2: real backend commands (connect_device、status snapshot、run start、safe shutdown)；强调 GUI 只发 user intent，executor 拥有 run authority，safety 拥有 allow/reject 权

## Code Structure

- **文件**: `src/routes/AboutBoundariesPage.tsx` (124 行)
- **数据**: 100% 硬编码 JSX，无任何外部数据源
- **组件**: 纯 CSS-in-JS，无外部 UI 库
- **交互**: 无状态、无事件、无 Tauri 命令
- **布局**: 全宽 statement 卡片 → 2 列 grid（Allowed / Forbidden）→ 全宽 Future Path 卡片

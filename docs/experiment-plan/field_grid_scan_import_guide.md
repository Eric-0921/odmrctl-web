# ODMR 磁场网格扫描导入配置指南

本目录的 1D/2D/3D 示例 JSON 可以直接在 GUI 中导入：

1. 打开 `实验计划`。
2. 点击 `导入实验 JSON`。
3. 选择 `examples/experiment_plans/odmr_field_grid_*.example.json`。
4. 查看 `Step 投影预览`、`谱线内部 RF 点预览` 和 `ODMR 谱线图`。
5. 点击 `用当前零场解析` 前，必须先完成设备预检、磁场测零和锁零。

## 核心语义

`Step = 一个磁场向量点下的一条 ODMR 谱线`。一条谱线内部包含 SMB100A RF frequency sweep 和 OE1022D 采集窗口；RF 点不是 Step。

1D/2D/3D 都是独立变量网格：

- 1D 单轴线扫描：只有一根轴扫描，其余轴固定。
- 2D 平面网格：两根轴分别扫描后做笛卡尔积，第三根轴固定。
- 3D 体网格：Bx、By、Bz 三根轴分别扫描后做笛卡尔积。

禁止把 2D/3D 理解成对角线扫描。`Bx=By=Bz=t` 不是本系统的三轴网格语义。

## JSON 字段

- `field_space.mode`: 必须是 `grouped_grid_scan`。
- `groups[].axes`: 参与扫描的轴，例如 `["x", "y"]`。
- `groups[].axis_ranges_nt`: 每个参与扫描轴自己的 `start / stop / step`。
- `groups[].fixed_axes_nt`: 未参与扫描轴的固定值，也可用于非零偏置切片。
- `spectrum_template.rf_sweep`: 每条 ODMR 谱线内部的 RF sweep。
- `spectrum_template.oe1022d_acquisition`: OE1022D 跟随 RF sweep 的采集窗口。
- `spectrum_template.laser`: 第一版固定功率条件。

示例默认使用 `0 -> 20 nT, step 10 nT`，用于安全演示和快速导入。真实实验可把范围改为例如 `0 -> 280000 nT, step 10 nT`。大规模点集不会全部展开到 Step 表，GUI 默认只预览前 200 条，并显示完整总点数。

## 自检点

导入 2D 示例后，投影结果应包含：

- `(10, 0, 0)`
- `(0, 10, 0)`
- `(10, 10, 0)`

导入 3D 示例后，投影结果应包含：

- `(10, 0, 0)`
- `(0, 10, 0)`
- `(0, 0, 10)`
- `(10, 10, 10)`

如果这些点在范围覆盖时不存在，说明生成器又退化成了对角线或稀疏采样，不能用于实验。

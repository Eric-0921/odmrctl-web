import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  DeviceDefaultPackage,
  ExperimentPlanProjection,
  ExperimentPlanRunStatus,
  ExperimentRunReadiness,
  ExperimentPlanSummary,
  ResolvedPlanPreview,
} from "../types/deviceWorkbench";

const cardStyle: React.CSSProperties = {
  background: "var(--color-surface)",
  border: "1px solid var(--color-border)",
  borderRadius: "var(--radius-md)",
  padding: "var(--space-4)",
  boxShadow: "var(--shadow-card)",
};

const btnPrimary: React.CSSProperties = {
  padding: "8px 14px",
  borderRadius: "var(--radius-sm)",
  border: "none",
  background: "var(--color-primary)",
  color: "#fff",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  cursor: "pointer",
};

const btnSecondary: React.CSSProperties = {
  padding: "8px 14px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  color: "var(--color-text)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  cursor: "pointer",
};

const btnDanger: React.CSSProperties = {
  ...btnSecondary,
  color: "var(--color-danger)",
  borderColor: "var(--color-danger)",
};

const inputStyle: React.CSSProperties = {
  padding: "5px 8px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  fontSize: "var(--font-size-xs)",
  width: "100%",
  fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
};

const monoBlock: React.CSSProperties = {
  background: "#14141f",
  color: "#d9e0ee",
  borderRadius: "var(--radius-sm)",
  padding: "var(--space-3)",
  fontSize: "var(--font-size-xs)",
  fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
  overflow: "auto",
  maxHeight: 520,
  lineHeight: 1.5,
};

const tableStyle: React.CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: "var(--font-size-xs)",
};

const thStyle: React.CSSProperties = {
  padding: 8,
  textAlign: "left",
  color: "var(--color-text-muted)",
  borderBottom: "1px solid var(--color-border)",
  whiteSpace: "nowrap",
};

const tdStyle: React.CSSProperties = {
  padding: 8,
  borderBottom: "1px solid var(--color-border)",
  verticalAlign: "top",
};

type EditorTab = "packages" | "scan" | "field" | "smb100a" | "oe1022d" | "laser" | "steps" | "json";
type FrequencyUnit = "Hz" | "kHz" | "MHz" | "GHz";
type VoltageUnit = "V" | "mV";
type MagneticAxis = "x" | "y" | "z";
type MagneticScanKind = "line_1d" | "plane_2d" | "volume_3d" | "custom_grid";
type PlanRecord = Record<string, unknown>;

interface StepDraftRow {
  id: string;
  groupId?: string;
  bxNt: string;
  byNt: string;
  bzNt: string;
  rfStart: string;
  rfStartUnit: FrequencyUnit;
  rfStop: string;
  rfStopUnit: FrequencyUnit;
  rfStep: string;
  rfStepUnit: FrequencyUnit;
  dwellMs: string;
  powerDbm: string;
  sweepOutputStart: string;
  sweepOutputStartUnit: VoltageUnit;
  sweepOutputStop: string;
  sweepOutputStopUnit: VoltageUnit;
  laserPowerMw: string;
  laserEnabled: boolean;
  oePreStartMs: string;
  oePostStopMs: string;
  chATimeConstantS: string;
  chAFilterSlope: string;
  chADynamicReserve: string;
  chASensitivity: string;
  chBTimeConstantS: string;
  chBFilterSlope: string;
  chBDynamicReserve: string;
  chBSensitivity: string;
}

interface MagneticScanGroupDraft {
  groupId: string;
  labelCn: string;
  scanKind: MagneticScanKind;
  axes: MagneticAxis[];
  enabled: boolean;
  axisRanges: Record<MagneticAxis, AxisRangeDraft>;
  fixedXNt: string;
  fixedYNt: string;
  fixedZNt: string;
  order: number;
  overrideEnabled: boolean;
  powerDbm: string;
  laserPowerMw: string;
  laserEnabled: boolean;
  oePreStartMs: string;
  oePostStopMs: string;
  chATimeConstantS: string;
  chAFilterSlope: string;
  chADynamicReserve: string;
  chASensitivity: string;
  chBTimeConstantS: string;
  chBFilterSlope: string;
  chBDynamicReserve: string;
  chBSensitivity: string;
}

interface AxisRangeDraft {
  startNt: string;
  stopNt: string;
  stepNt: string;
}

const defaultRow = (id = "spectrum_0000"): StepDraftRow => ({
  id,
  bxNt: "0",
  byNt: "0",
  bzNt: "0",
  rfStart: "2.8",
  rfStartUnit: "GHz",
  rfStop: "2.9",
  rfStopUnit: "GHz",
  rfStep: "1",
  rfStepUnit: "MHz",
  dwellMs: "300",
  powerDbm: "-30",
  sweepOutputStart: "0",
  sweepOutputStartUnit: "V",
  sweepOutputStop: "3",
  sweepOutputStopUnit: "V",
  laserPowerMw: "",
  laserEnabled: false,
  oePreStartMs: "50",
  oePostStopMs: "50",
  chATimeConstantS: "0.3",
  chAFilterSlope: "12",
  chADynamicReserve: "正常 (NORMAL)",
  chASensitivity: "100 mV/nA",
  chBTimeConstantS: "0.3",
  chBFilterSlope: "12",
  chBDynamicReserve: "正常 (NORMAL)",
  chBSensitivity: "100 mV/nA",
});

const defaultScanGroups = (): MagneticScanGroupDraft[] => {
  const specs: Array<[string, string, MagneticScanKind, MagneticAxis[]]> = [
    ["bx_line", "Bx 单轴", "line_1d", ["x"]],
    ["by_line", "By 单轴", "line_1d", ["y"]],
    ["bz_line", "Bz 单轴", "line_1d", ["z"]],
    ["bx_by_plane", "Bx-By 二维网格", "plane_2d", ["x", "y"]],
    ["by_bz_plane", "By-Bz 二维网格", "plane_2d", ["y", "z"]],
    ["bx_bz_plane", "Bx-Bz 二维网格", "plane_2d", ["x", "z"]],
    ["bx_by_bz_volume", "Bx-By-Bz 三维网格", "volume_3d", ["x", "y", "z"]],
  ];
  return specs.map(([groupId, labelCn, scanKind, axes], index) => ({
    groupId,
    labelCn,
    scanKind,
    axes,
    enabled: index === 0,
    axisRanges: {
      x: { startNt: "0", stopNt: "1000", stepNt: "10" },
      y: { startNt: "0", stopNt: "1000", stepNt: "10" },
      z: { startNt: "0", stopNt: "1000", stepNt: "10" },
    },
    fixedXNt: "0",
    fixedYNt: "0",
    fixedZNt: "0",
    order: index,
    overrideEnabled: false,
    powerDbm: "",
    laserPowerMw: "",
    laserEnabled: false,
    oePreStartMs: "",
    oePostStopMs: "",
    chATimeConstantS: "",
    chAFilterSlope: "",
    chADynamicReserve: "",
    chASensitivity: "",
    chBTimeConstantS: "",
    chBFilterSlope: "",
    chBDynamicReserve: "",
    chBSensitivity: "",
  }));
};

const customScanGroup = (index: number): MagneticScanGroupDraft => ({
  ...defaultScanGroups()[0],
  groupId: `custom_grid_${index.toString().padStart(2, "0")}`,
  labelCn: "自定义网格组",
  scanKind: "custom_grid",
  axes: ["x", "y"],
  enabled: true,
  order: index,
});

const scanKindLabel = (kind: MagneticScanKind) => {
  switch (kind) {
    case "line_1d":
      return "单轴 line";
    case "plane_2d":
      return "二维网格 plane";
    case "volume_3d":
      return "三维网格 volume";
    default:
      return "自定义 grid";
  }
};

const hzFactor: Record<FrequencyUnit, number> = { Hz: 1, kHz: 1e3, MHz: 1e6, GHz: 1e9 };
const voltageFactor: Record<VoltageUnit, number> = { V: 1, mV: 1e-3 };
const frequencyUnits: FrequencyUnit[] = ["Hz", "kHz", "MHz", "GHz"];
const voltageUnits: VoltageUnit[] = ["V", "mV"];
const reserveOptions = ["低噪声 (LNOise)", "正常 (NORMAL)", "高储备 (HIGH)"];
const slopeOptions = ["6", "12", "18", "24"];
const sensitivityOptions = ["10 uV/nA", "30 uV/nA", "100 uV/nA", "300 uV/nA", "1 mV/nA", "3 mV/nA", "10 mV/nA", "30 mV/nA", "100 mV/nA", "300 mV/nA", "1 V/nA"];

const fallbackDefaultPackages: DeviceDefaultPackage[] = [
  {
    device: "smb100a",
    package_id: "safe_defaults",
    label_cn: "SMB100A 安全默认配置组",
    source: "examples/面板基础配置-oe1022d/smb100a/*_checked_v2.json safe_value",
    risk_level: "safe",
    values: { frequency_hz: 1_000_000, power_dbm: -30, rf_output: "OFF", mod_state: "OFF" },
    values_si: { frequency_hz: 1_000_000, power_dbm: -30, rf_output: "OFF", mod_state: "OFF" },
    note_cn: "用于连接、预检和非采集阶段；RF 输出与调制默认关闭。",
    apply_target: "device_preset_draft",
  },
  {
    device: "oe1022d",
    package_id: "panel_current_defaults",
    label_cn: "OE1022D 面板当前值配置组",
    source: "docs/equipment_manual/oe1022d/校对后的oe1022d面板基础设置/*_checked_v2.json",
    risk_level: "pass",
    values: { time_constant: "300 ms", filter_slope: "12 dB/oct", sensitivity: "100 mV/nA" },
    values_si: {
      oe1022d_acquisition: {
        mode: "follow_rf_sweep",
        pre_start_ms: 50,
        post_stop_ms: 50,
        channels: {
          ch_a: { time_constant_s: 0.3, filter_slope_db_oct: 12, dynamic_reserve: "NORMAL", sensitivity: "100 mV/nA" },
          ch_b: { time_constant_s: 0.3, filter_slope_db_oct: 12, dynamic_reserve: "NORMAL", sensitivity: "100 mV/nA" },
        },
      },
    },
    note_cn: "来自旧 GUI 截图当前值，适合作为采集模板草稿。",
    apply_target: "device_preset_draft",
  },
];

const badge = (state: "ok" | "warning" | "blocked" | "off" | "on"): React.CSSProperties => {
  const palette = {
    ok: ["var(--color-success-soft)", "var(--color-success)"],
    warning: ["#fff7ed", "#c2410c"],
    blocked: ["var(--color-danger-soft)", "var(--color-danger)"],
    off: ["var(--color-disabled-bg)", "var(--color-disabled-text)"],
    on: ["#dbeafe", "var(--color-primary)"],
  } as const;
  const [background, color] = palette[state];
  return {
    display: "inline-block",
    padding: "2px 8px",
    borderRadius: "var(--radius-sm)",
    fontSize: "var(--font-size-xs)",
    fontWeight: 600,
    background,
    color,
  };
};

const runStateBadge = (state: string): "ok" | "warning" | "blocked" | "off" | "on" => {
  if (state === "completed") return "ok";
  if (state === "running") return "on";
  if (state === "blocked" || state === "failed") return "blocked";
  if (state === "stop_requested") return "warning";
  return "off";
};

const parseNumber = (value: string, fallback = 0) => {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : fallback;
};

const optionalNumber = (value: string) => {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : undefined;
};

const toHz = (value: string, unit: FrequencyUnit) => parseNumber(value) * hzFactor[unit];
const toVolt = (value: string, unit: VoltageUnit) => parseNumber(value) * voltageFactor[unit];
const fromHz = (hz: unknown, unit: FrequencyUnit) => (typeof hz === "number" ? String(hz / hzFactor[unit]) : "");
const fmtHz = (hz?: number | null) => (hz == null ? "—" : `${(hz / 1e9).toFixed(6)} GHz`);
const riskState = (risk: string): "ok" | "warning" | "blocked" | "off" | "on" => {
  if (risk === "warning") return "warning";
  if (risk === "blocked") return "blocked";
  if (risk === "safe" || risk === "pass") return "ok";
  return "off";
};

const isRecord = (value: unknown): value is PlanRecord => typeof value === "object" && value !== null && !Array.isArray(value);
const asRecord = (value: unknown): PlanRecord => isRecord(value) ? value : {};
const getArray = (value: unknown, key: string): unknown[] => {
  if (!isRecord(value)) return [];
  const child = value[key];
  return Array.isArray(child) ? child : [];
};

export default function ExperimentPlanPage() {
  const [summary, setSummary] = useState<ExperimentPlanSummary | null>(null);
  const [resolved, setResolved] = useState<ResolvedPlanPreview | null>(null);
  const [projection, setProjection] = useState<ExperimentPlanProjection | null>(null);
  const [draftJson, setDraftJson] = useState<unknown>(null);
  const [planDraft, setPlanDraft] = useState<PlanRecord | null>(null);
  const [selectedPackages, setSelectedPackages] = useState<Record<string, string>>({});
  const [runReadiness, setRunReadiness] = useState<ExperimentRunReadiness | null>(null);
  const [runStatus, setRunStatus] = useState<ExperimentPlanRunStatus | null>(null);
  const [runBusy, setRunBusy] = useState<string | null>(null);
  const [operatorConfirmed, setOperatorConfirmed] = useState(false);
  const [activeTab, setActiveTab] = useState<EditorTab>("packages");
  const [stepRows, setStepRows] = useState<StepDraftRow[]>([defaultRow()]);
  const [scanGroups, setScanGroups] = useState<MagneticScanGroupDraft[]>(defaultScanGroups);
  const [error, setError] = useState<string | null>(null);

  const defaultPackages = projection?.default_packages.length ? projection.default_packages : fallbackDefaultPackages;

  const run = async <T,>(fn: () => Promise<T>) => {
    setError(null);
    try {
      return await fn();
    } catch (e) {
      setError(String(e));
      return null;
    }
  };

  const refreshProjection = async () => {
    const projected = await run(() => invoke<ExperimentPlanProjection>("project_experiment_plan"));
    if (projected) setProjection(projected);
  };

  const refreshRunStatus = async () => {
    const status = await run(() => invoke<ExperimentPlanRunStatus | null>("get_experiment_plan_run_status"));
    if (status) setRunStatus(status);
  };

  const checkRunReadiness = async () => {
    setRunBusy("readiness");
    const readiness = await run(() => invoke<ExperimentRunReadiness>("get_experiment_plan_run_readiness"));
    if (readiness) setRunReadiness(readiness);
    setRunBusy(null);
  };

  const startPreviewRun = async () => {
    setRunBusy("preview");
    const status = await run(() => invoke<ExperimentPlanRunStatus>("start_experiment_plan_run", { mode: "preview", operatorConfirmed: true }));
    if (status) setRunStatus(status);
    const readiness = await run(() => invoke<ExperimentRunReadiness>("get_experiment_plan_run_readiness"));
    if (readiness) setRunReadiness(readiness);
    setRunBusy(null);
  };

  const requestHardwareRun = async () => {
    setRunBusy("hardware");
    const status = await run(() => invoke<ExperimentPlanRunStatus>("start_experiment_plan_run", { mode: "hardware", operatorConfirmed }));
    if (status) setRunStatus(status);
    const readiness = await run(() => invoke<ExperimentRunReadiness>("get_experiment_plan_run_readiness"));
    if (readiness) setRunReadiness(readiness);
    setRunBusy(null);
  };

  const stopRun = async () => {
    setRunBusy("stop");
    const status = await run(() => invoke<ExperimentPlanRunStatus>("stop_experiment_plan_run"));
    if (status) setRunStatus(status);
    setRunBusy(null);
  };

  useEffect(() => {
    let cancelled = false;
    const loadSession = async () => {
      const draft = await run(() => invoke<unknown>("get_experiment_plan_draft"));
      const selected = await run(() => invoke<Record<string, string>>("get_selected_default_packages"));
      if (cancelled) return;
      if (isRecord(draft)) {
        setPlanDraft(draft);
        setDraftJson(draft);
        setStepRows(rowsFromDraft(draft));
        setScanGroups(scanGroupsFromDraft(draft));
      }
      if (selected) setSelectedPackages(selected);
      await refreshProjection();
      await refreshRunStatus();
    };
    void loadSession();
    return () => {
      cancelled = true;
    };
  }, []);

  const loadPlan = async () => {
    const path = await run(() => invoke<string | null>("pick_recipe_file"));
    if (!path) return;
    const loaded = await run(() => invoke<ExperimentPlanSummary>("load_experiment_plan", { path }));
    if (!loaded) return;
    const raw = loaded.raw as PlanRecord;
    setSummary(loaded);
    setResolved(null);
    setDraftJson(raw);
    setPlanDraft(raw);
    setStepRows(rowsFromDraft(raw));
    setScanGroups(scanGroupsFromDraft(raw));
    await refreshProjection();
  };

  const capturePreset = async () => {
    const value = await run(() => invoke<unknown>("capture_current_setup_as_preset_draft"));
    if (value) setDraftJson(value);
  };

  const capturePlan = async () => {
    const value = await run(() => invoke<unknown>("capture_current_setup_as_plan_draft"));
    if (!value) return;
    setDraftJson(value);
    if (isRecord(value)) {
      setPlanDraft(value);
      setStepRows(rowsFromDraft(value));
      setScanGroups(scanGroupsFromDraft(value));
    }
  };

  const resolvePlan = async () => {
    const preview = await run(() => invoke<ResolvedPlanPreview>("resolve_plan_with_current_zero"));
    if (preview) setResolved(preview);
  };

  const exportPlanJson = async () => {
    await applyDraft();
    const path = await run(() => invoke<string | null>("export_experiment_plan_json"));
    if (path) setError(`已导出实验计划 JSON：${path}`);
  };

  const applyDraft = async (rows = stepRows, base = planDraft) => {
    const next = buildDraftJson(base, rows, scanGroups);
    const updated = await run(() => invoke<ExperimentPlanSummary>("set_experiment_plan_draft", { plan: next }));
    if (!updated) return;
    setSummary(updated);
    setPlanDraft(next);
    setDraftJson(next);
    setResolved(null);
    await refreshProjection();
  };

  const addStep = () => {
    setStepRows((rows) => [...rows, { ...(rows[rows.length - 1] ?? defaultRow()), id: `spectrum_${rows.length.toString().padStart(4, "0")}` }]);
    setActiveTab("steps");
  };

  const duplicateStep = (index: number) => {
    setStepRows((rows) => {
      const row = rows[index];
      if (!row) return rows;
      const next = [...rows];
      next.splice(index + 1, 0, { ...row, id: `spectrum_${next.length.toString().padStart(4, "0")}` });
      return next;
    });
  };

  const deleteStep = (index: number) => {
    setStepRows((rows) => rows.length <= 1 ? rows : rows.filter((_, i) => i !== index));
  };

  const generateDemoSweep = () => {
    setStepRows([
      { ...defaultRow("demo_x_0000"), bxNt: "0", byNt: "0", bzNt: "0" },
      { ...defaultRow("demo_x_0001"), bxNt: "500", byNt: "0", bzNt: "0" },
      { ...defaultRow("demo_x_0002"), bxNt: "1000", byNt: "0", bzNt: "0" },
    ]);
    setActiveTab("steps");
  };

  const previewScanGroups = () => {
    const previewRows = rowsFromScanGroups(scanGroups, stepRows[0] ?? defaultRow(), 200);
    setStepRows(previewRows.length > 0 ? previewRows : [defaultRow()]);
    setActiveTab("steps");
  };

  const applyScanGroups = async () => {
    const previewRows = rowsFromScanGroups(scanGroups, stepRows[0] ?? defaultRow(), 200);
    if (previewRows.length > 0) setStepRows(previewRows);
    await applyDraft(previewRows.length > 0 ? previewRows : stepRows, planDraft);
  };

  const applyPackage = async (pkg: DeviceDefaultPackage) => {
    const values = isRecord(pkg.values_si) ? pkg.values_si : {};
    const updatedRows = applyPackageToRows(stepRows, pkg.device, values);
    const nextBase = mergePackageIntoDraft(planDraft, pkg);
    setStepRows(updatedRows);
    setPlanDraft(nextBase);
    setDraftJson(nextBase);
    setSelectedPackages((prev) => ({ ...prev, [pkg.device]: pkg.package_id }));
    await run(() => invoke<Record<string, string>>("set_selected_default_package", { device: pkg.device, packageId: pkg.package_id }));
    await run(() => invoke<Record<string, unknown>>("set_device_preset_draft", { device: pkg.device, draft: pkg.values_si }));
    await applyDraft(updatedRows, nextBase);
  };

  const projectedJson = draftJson ?? resolved ?? summary?.raw ?? planDraft;

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>实验计划</h1>

      <div style={{ ...cardStyle, marginBottom: "var(--space-4)" }}>
        <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
          <button onClick={loadPlan} style={btnPrimary}>导入实验计划 JSON</button>
          <button onClick={resolvePlan} style={btnSecondary}>按当前零场计算电流预览</button>
          <button onClick={capturePreset} style={btnSecondary}>从当前设备生成参数模板</button>
          <button onClick={capturePlan} style={btnSecondary}>用当前设置新建实验计划</button>
          <button onClick={exportPlanJson} style={btnSecondary}>导出计划 JSON</button>
          <button onClick={generateDemoSweep} style={btnSecondary}>生成 3 条 demo 谱线</button>
        </div>
        {error && <div style={{ marginTop: "var(--space-3)", color: "var(--color-danger)", fontSize: "var(--font-size-sm)" }}>{error}</div>}
      </div>

      <RunLauncherPanel
        readiness={runReadiness}
        status={runStatus}
        busy={runBusy}
        onCheck={checkRunReadiness}
        onPreview={startPreviewRun}
        onHardware={requestHardwareRun}
        onStop={stopRun}
        operatorConfirmed={operatorConfirmed}
        onOperatorConfirmed={setOperatorConfirmed}
      />

      <div style={{ display: "grid", gridTemplateColumns: "320px minmax(0, 1fr)", gap: "var(--space-4)", alignItems: "start" }}>
        <PlanSummaryCard summary={summary} resolved={resolved} projection={projection} />
        <div style={{ display: "grid", gap: "var(--space-4)", minWidth: 0 }}>
          <div>
            <h2 style={{ fontSize: "var(--font-size-xl)", margin: 0 }}>实验步骤草稿编辑器</h2>
            <p style={{ margin: "4px 0 0", color: "var(--color-text-muted)", fontSize: "var(--font-size-sm)" }}>
              一行谱线步骤表示一个磁场向量点下的一条 ODMR 谱线；RF sweep 是谱线内部动作，不再作为单独步骤展示。
            </p>
          </div>
          <EditorTabs activeTab={activeTab} onChange={setActiveTab} />
          <div style={cardStyle}>
            {activeTab === "packages" && <DefaultPackageSelector packages={defaultPackages} selected={selectedPackages} onApply={applyPackage} />}
            {activeTab === "scan" && (
              <MagneticScanGenerator
                groups={scanGroups}
                setGroups={setScanGroups}
                templateRow={stepRows[0] ?? defaultRow()}
                onPreview={previewScanGroups}
                onApply={() => void applyScanGroups()}
              />
            )}
            {activeTab === "field" && <FieldEditor rows={stepRows} setRows={setStepRows} />}
            {activeTab === "smb100a" && <SmbTemplateEditor rows={stepRows} setRows={setStepRows} />}
            {activeTab === "oe1022d" && <OeTemplateEditor rows={stepRows} setRows={setStepRows} />}
            {activeTab === "laser" && <LaserEditor rows={stepRows} setRows={setStepRows} />}
            {activeTab === "steps" && (
              <StepEditor
                rows={stepRows}
                setRows={setStepRows}
                onAdd={addStep}
                onDuplicate={duplicateStep}
                onDelete={deleteStep}
                projection={projection}
                resolved={resolved}
              />
            )}
            {activeTab === "json" && <JsonView value={projectedJson ?? { message: "尚未导入或生成 JSON" }} />}
          </div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "var(--space-3)" }}>
            <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)" }}>
              表格编辑只更新 Tauri 会话中的 experiment JSON draft，不会控制已连接设备。
            </div>
            <button onClick={() => void applyDraft()} style={btnPrimary}>保存表格修改到计划</button>
          </div>
          <DeviceParameterTables projection={projection} resolved={resolved} />
        </div>
      </div>
    </div>
  );
}

function RunLauncherPanel({
  readiness,
  status,
  busy,
  onCheck,
  onPreview,
  onHardware,
  onStop,
  operatorConfirmed,
  onOperatorConfirmed,
}: {
  readiness: ExperimentRunReadiness | null;
  status: ExperimentPlanRunStatus | null;
  busy: string | null;
  onCheck: () => void;
  onPreview: () => void;
  onHardware: () => void;
  onStop: () => void;
  operatorConfirmed: boolean;
  onOperatorConfirmed: (confirmed: boolean) => void;
}) {
  const previewReady = readiness?.ready_for_preview_execution ?? false;
  const hardwareReady = readiness?.ready_for_hardware_execution ?? false;
  const blocking = readiness?.hardware_blocked_reasons ?? status?.blocked_reasons ?? [];
  return (
    <div style={{ ...cardStyle, marginBottom: "var(--space-4)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: "var(--space-3)", alignItems: "start", flexWrap: "wrap" }}>
        <div>
          <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-1)" }}>执行控制</h2>
          <p style={{ margin: 0, color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)" }}>
            先生成执行预案，再启动真实采集。真实采集必须通过后端 typed command 与 executor handoff，不允许前端直接访问硬件。
          </p>
        </div>
        <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap", justifyContent: "end" }}>
          <button onClick={onCheck} disabled={busy != null} style={btnSecondary}>{busy === "readiness" ? "检查中..." : "执行前检查"}</button>
          <button onClick={onPreview} disabled={busy != null || (readiness != null && !previewReady)} style={btnPrimary}>{busy === "preview" ? "生成中..." : "生成执行预案"}</button>
          <button onClick={onHardware} disabled={busy != null || !operatorConfirmed} style={btnDanger}>{busy === "hardware" ? "启动中..." : "启动真实采集"}</button>
          <button onClick={onStop} disabled={busy != null} style={btnSecondary}>{busy === "stop" ? "停止中..." : "停止采集并清理"}</button>
        </div>
      </div>
      <label style={{ display: "flex", alignItems: "center", gap: 8, marginTop: "var(--space-3)", fontSize: "var(--font-size-xs)", color: "var(--color-text)" }}>
        <input
          type="checkbox"
          checked={operatorConfirmed}
          onChange={(event) => onOperatorConfirmed(event.target.checked)}
        />
        我确认 RF、激光、磁场和长时间运行条件已检查，允许启动真实采集。
      </label>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "var(--space-3)", marginTop: "var(--space-3)", fontSize: "var(--font-size-xs)" }}>
        <div><strong>预览执行:</strong> <span style={badge(previewReady ? "ok" : "off")}>{previewReady ? "可启动" : "未检查/阻塞"}</span></div>
        <div><strong>真实硬件执行:</strong> <span style={badge(hardwareReady ? "ok" : "blocked")}>{hardwareReady ? "可启动" : "阻塞"}</span></div>
        <div><strong>Step:</strong> {readiness?.step_count ?? status?.step_count ?? "—"}</div>
        <div><strong>RF 点/谱线:</strong> {readiness?.rf_point_count ?? status?.rf_point_count ?? "—"}</div>
          <div><strong>总测量点:</strong> {readiness?.estimated_measurements ?? status?.estimated_measurements ?? "—"}</div>
          <div><strong>估算耗时:</strong> {readiness?.estimated_duration_s != null ? `${readiness.estimated_duration_s.toFixed(1)} s` : status?.estimated_duration_s != null ? `${status.estimated_duration_s.toFixed(1)} s` : "—"}</div>
          <div><strong>当前阶段:</strong> {status?.current_phase ?? "—"}</div>
          <div><strong>当前 B:</strong> {status?.current_b_nt ? `[${status.current_b_nt.join(", ")}] nT` : "—"}</div>
          <div><strong>SMB sweep:</strong> {status?.smb_sweep_running ? "运行中" : "未运行"}</div>
          <div><strong>OE 帧:</strong> {status?.oe_frames_captured ?? "—"}</div>
          <div><strong>Cleanup:</strong> {status?.cleanup_state ?? "—"}</div>
      </div>
      {readiness && (
        <div style={{ marginTop: "var(--space-3)", fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)" }}>
          <strong>已连接/锁定:</strong> {readiness.connected_devices.length > 0 ? readiness.connected_devices.join(", ") : "无"} · <strong>需要设备:</strong> {readiness.required_devices.join(", ")}
        </div>
      )}
      {status && (
        <div style={{ marginTop: "var(--space-3)", paddingTop: "var(--space-3)", borderTop: "1px solid var(--color-border)", fontSize: "var(--font-size-xs)" }}>
          <div><strong>最近执行:</strong> {status.run_id} · <span style={badge(runStateBadge(status.state))}>{status.state}</span> · {status.mode}</div>
          <div><strong>完成 Step:</strong> {status.steps_completed}/{status.step_count}</div>
          {status.run_directory && <div><strong>产物目录:</strong> <code>{status.run_directory}</code></div>}
          {status.recent_error && <div style={{ color: "var(--color-danger)" }}><strong>最近错误:</strong> {status.recent_error}</div>}
        </div>
      )}
      {readiness?.warnings.length ? (
        <div style={{ marginTop: "var(--space-3)", color: "var(--color-warning)", fontSize: "var(--font-size-xs)" }}>
          {readiness.warnings.join(" · ")}
        </div>
      ) : null}
      {blocking.length > 0 && (
        <div style={{ marginTop: "var(--space-3)", color: "var(--color-danger)", fontSize: "var(--font-size-xs)" }}>
          {blocking.join(" · ")}
        </div>
      )}
    </div>
  );
}

function PlanSummaryCard({
  summary,
  resolved,
  projection,
}: {
  summary: ExperimentPlanSummary | null;
  resolved: ResolvedPlanPreview | null;
  projection: ExperimentPlanProjection | null;
}) {
  return (
    <div style={cardStyle}>
      <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-3)" }}>计划投影</h2>
      <div style={{ display: "grid", gap: "var(--space-2)", fontSize: "var(--font-size-sm)" }}>
        <div><strong>ID:</strong> {summary?.id ?? "未命名草稿"}</div>
        <div><strong>类型:</strong> {summary?.kind ?? projection?.kind ?? "experiment_plan"}</div>
        <div><strong>谱线 Step:</strong> {projection?.step_row_count ?? summary?.field_point_count ?? 0}</div>
        <div><strong>RF 点/谱线:</strong> {summary?.rf_point_count ?? projection?.smb100a_rf_points.length ?? 0}</div>
        <div><strong>总测量点:</strong> {projection?.estimated_measurements ?? summary?.estimated_measurements ?? 0}</div>
        <div><strong>零场锁定:</strong> {summary?.require_zero_lock ?? true ? "需要" : "不需要"}</div>
        {projection?.estimated_duration_s != null && <div><strong>估算耗时:</strong> {projection.estimated_duration_s.toFixed(1)} s</div>}
        {projection?.truncated && <div style={{ color: "var(--color-warning)" }}>仅预览前 {projection.preview_limit} 行。</div>}
      </div>
      {resolved && (
        <div style={{ marginTop: "var(--space-4)", paddingTop: "var(--space-4)", borderTop: "1px solid var(--color-border)" }}>
          <span style={badge(resolved.executable ? "ok" : "blocked")}>{resolved.executable ? "可执行预览" : "阻塞"}</span>
          {resolved.blocked_reasons.length > 0 && (
            <div style={{ marginTop: "var(--space-2)", color: "var(--color-danger)", fontSize: "var(--font-size-xs)" }}>
              {resolved.blocked_reasons.join(" · ")}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function EditorTabs({ activeTab, onChange }: { activeTab: EditorTab; onChange: (tab: EditorTab) => void }) {
  const tabs: Array<[EditorTab, string]> = [
    ["packages", "设备模板"],
    ["scan", "磁场扫描"],
    ["field", "磁场"],
    ["smb100a", "SMB100A"],
    ["oe1022d", "OE1022D"],
    ["laser", "激光"],
    ["steps", "谱线步骤"],
    ["json", "计划 JSON"],
  ];
  return (
    <div style={{ ...cardStyle, display: "flex", gap: "var(--space-2)", flexWrap: "wrap", padding: "var(--space-3)" }}>
      {tabs.map(([tab, label]) => <button key={tab} onClick={() => onChange(tab)} style={activeTab === tab ? btnPrimary : btnSecondary}>{label}</button>)}
    </div>
  );
}

function DefaultPackageSelector({
  packages,
  selected,
  onApply,
}: {
  packages: DeviceDefaultPackage[];
  selected: Record<string, string>;
  onApply: (pkg: DeviceDefaultPackage) => void;
}) {
  const groups = useMemo(() => groupBy(packages, (pkg) => pkg.device), [packages]);
  return (
    <div>
      <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>预设配置组选择器</h2>
      <div style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)", marginBottom: "var(--space-3)" }}>
        套用只写入 draft JSON 和会话态参数包，不会应用到设备；真实下发必须去设备工作台点击“应用到设备”。
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "var(--space-3)" }}>
        {Object.entries(groups).map(([device, devicePackages]) => (
          <div key={device} style={{ border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", padding: "var(--space-3)" }}>
            <h3 style={{ fontSize: "var(--font-size-md)", marginBottom: "var(--space-2)" }}>{device}</h3>
            {devicePackages.map((pkg) => (
              <details key={pkg.package_id} style={{ marginBottom: "var(--space-2)" }}>
                <summary style={{ cursor: "pointer", fontWeight: 600 }}>
                  {pkg.label_cn}{" "}
                  <span style={badge(riskState(pkg.risk_level))}>{pkg.risk_level}</span>{" "}
                  {selected[device] === pkg.package_id && <span style={badge("on")}>已选</span>}
                </summary>
                <div style={{ marginTop: "var(--space-2)", display: "grid", gap: 6, fontSize: "var(--font-size-xs)" }}>
                  <div><strong>来源:</strong> {pkg.source}</div>
                  <div><strong>说明:</strong> {pkg.note_cn}</div>
                  <div><strong>目标:</strong> {pkg.apply_target}</div>
                  <pre style={{ ...monoBlock, maxHeight: 180 }}>{JSON.stringify(pkg.values_si, null, 2)}</pre>
                  <button onClick={() => onApply(pkg)} style={btnPrimary}>套用到草稿</button>
                </div>
              </details>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

function MagneticScanGenerator({
  groups,
  setGroups,
  templateRow,
  onPreview,
  onApply,
}: {
  groups: MagneticScanGroupDraft[];
  setGroups: React.Dispatch<React.SetStateAction<MagneticScanGroupDraft[]>>;
  templateRow: StepDraftRow;
  onPreview: () => void;
  onApply: () => void;
}) {
  const totalPoints = groups.reduce((sum, group) => sum + groupPointCount(group), 0);
  const enabledCount = groups.filter((group) => group.enabled).length;
  const patchGroup = (groupId: string, patch: Partial<MagneticScanGroupDraft>) => {
    setGroups((current) => current.map((group) => group.groupId === groupId ? { ...group, ...patch } : group));
  };
  const patchAxisRange = (group: MagneticScanGroupDraft, axis: MagneticAxis, patch: Partial<AxisRangeDraft>) => {
    patchGroup(group.groupId, {
      axisRanges: {
        ...group.axisRanges,
        [axis]: { ...group.axisRanges[axis], ...patch },
      },
    });
  };
  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "start", gap: "var(--space-3)", marginBottom: "var(--space-3)" }}>
        <div>
          <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-1)" }}>磁场扫描生成器</h2>
          <p style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)" }}>
            外层变量是磁场；单轴是 line，二维/三维是笛卡尔积网格，全组合生成 ODMR 谱线 Step。RF sweep 内部频点不会变成 Step。
          </p>
          <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap", marginTop: "var(--space-2)" }}>
            <span style={badge(enabledCount > 0 ? "on" : "off")}>启用组 {enabledCount}</span>
            <span style={badge(totalPoints > 200 ? "warning" : "ok")}>预计谱线 Step {totalPoints}</span>
            {totalPoints > 200 && <span style={badge("warning")}>Step 表只预览前 200 行</span>}
          </div>
        </div>
        <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap", justifyContent: "end" }}>
          <button onClick={() => setGroups(defaultScanGroups().filter((group) => group.scanKind === "line_1d"))} style={btnSecondary}>单轴扫描</button>
          <button onClick={() => setGroups(defaultScanGroups().filter((group) => group.scanKind === "plane_2d"))} style={btnSecondary}>二维平面网格</button>
          <button onClick={() => setGroups(defaultScanGroups().filter((group) => group.scanKind === "volume_3d"))} style={btnSecondary}>三维体网格</button>
          <button onClick={() => setGroups((current) => [...current, customScanGroup(current.length)])} style={btnSecondary}>自定义网格组</button>
          <button onClick={onPreview} style={btnSecondary}>生成 Step 预览</button>
          <button onClick={onApply} style={btnPrimary}>应用扫描到 JSON 草稿</button>
        </div>
      </div>
      <div style={{ overflowX: "auto" }}>
        <table style={{ ...tableStyle, minWidth: 1500 }}>
          <thead>
            <tr>
              <th style={thStyle}>启用</th>
              <th style={thStyle}>组</th>
              <th style={thStyle}>轴集合</th>
              <th style={thStyle}>Bx range / fixed</th>
              <th style={thStyle}>By range / fixed</th>
              <th style={thStyle}>Bz range / fixed</th>
              <th style={thStyle}>点数公式</th>
              <th style={thStyle}>顺序</th>
              <th style={thStyle}>点数</th>
              <th style={thStyle}>组级覆盖</th>
            </tr>
          </thead>
          <tbody>
            {groups.map((group) => (
              <tr key={group.groupId}>
                <td style={tdStyle}><input type="checkbox" checked={group.enabled} onChange={(event) => patchGroup(group.groupId, { enabled: event.target.checked })} /></td>
                <td style={tdStyle}>
                  <input value={group.labelCn} onChange={(event) => patchGroup(group.groupId, { labelCn: event.target.value })} style={{ ...inputStyle, minWidth: 110 }} />
                  <div style={{ color: "var(--color-text-muted)", marginTop: 4 }}>{group.groupId} · {scanKindLabel(group.scanKind)}</div>
                </td>
                <td style={tdStyle}>{group.axes.map((axis) => axis.toUpperCase()).join(" × ")}</td>
                <AxisRangeCell group={group} axis="x" onRange={(patch) => patchAxisRange(group, "x", patch)} onFixed={(value) => patchGroup(group.groupId, { fixedXNt: value })} />
                <AxisRangeCell group={group} axis="y" onRange={(patch) => patchAxisRange(group, "y", patch)} onFixed={(value) => patchGroup(group.groupId, { fixedYNt: value })} />
                <AxisRangeCell group={group} axis="z" onRange={(patch) => patchAxisRange(group, "z", patch)} onFixed={(value) => patchGroup(group.groupId, { fixedZNt: value })} />
                <td style={tdStyle}>{groupPointFormula(group)}</td>
                <td style={tdStyle}><input value={String(group.order)} onChange={(event) => patchGroup(group.groupId, { order: Math.trunc(parseNumber(event.target.value, group.order)) })} style={{ ...inputStyle, minWidth: 70 }} /></td>
                <td style={tdStyle}>{groupPointCount(group)}</td>
                <td style={tdStyle}>
                  <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
                    <input type="checkbox" checked={group.overrideEnabled} onChange={(event) => patchGroup(group.groupId, { overrideEnabled: event.target.checked })} />
                    覆盖此组模板
                  </label>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "var(--space-3)", marginTop: "var(--space-4)" }}>
        {groups.filter((group) => group.overrideEnabled).map((group) => (
          <details key={`${group.groupId}.override`} open style={{ border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", padding: "var(--space-3)" }}>
            <summary style={{ cursor: "pointer", fontWeight: 700 }}>{group.labelCn} 组级覆盖</summary>
            <div style={{ display: "grid", gap: "var(--space-2)", marginTop: "var(--space-3)" }}>
              <LabelInput label={`SMB100A 功率 dBm，空则沿用 ${templateRow.powerDbm}`} value={group.powerDbm} onChange={(value) => patchGroup(group.groupId, { powerDbm: value })} />
              <LabelInput label="Laser 功率 mW" value={group.laserPowerMw} onChange={(value) => patchGroup(group.groupId, { laserPowerMw: value })} />
              <label style={{ display: "flex", gap: 8, alignItems: "center", fontSize: "var(--font-size-xs)" }}>
                <input type="checkbox" checked={group.laserEnabled} onChange={(event) => patchGroup(group.groupId, { laserEnabled: event.target.checked })} />
                覆盖启用 Laser
              </label>
              <LabelInput label="OE pre_start_ms" value={group.oePreStartMs} onChange={(value) => patchGroup(group.groupId, { oePreStartMs: value })} />
              <LabelInput label="OE post_stop_ms" value={group.oePostStopMs} onChange={(value) => patchGroup(group.groupId, { oePostStopMs: value })} />
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-2)" }}>
                <LabelInput label="Ch-A TC s" value={group.chATimeConstantS} onChange={(value) => patchGroup(group.groupId, { chATimeConstantS: value })} />
                <LabelInput label="Ch-A Slope" value={group.chAFilterSlope} onChange={(value) => patchGroup(group.groupId, { chAFilterSlope: value })} />
                <LabelInput label="Ch-B TC s" value={group.chBTimeConstantS} onChange={(value) => patchGroup(group.groupId, { chBTimeConstantS: value })} />
                <LabelInput label="Ch-B Slope" value={group.chBFilterSlope} onChange={(value) => patchGroup(group.groupId, { chBFilterSlope: value })} />
              </div>
            </div>
          </details>
        ))}
      </div>
    </div>
  );
}

function AxisRangeCell({
  group,
  axis,
  onRange,
  onFixed,
}: {
  group: MagneticScanGroupDraft;
  axis: MagneticAxis;
  onRange: (patch: Partial<AxisRangeDraft>) => void;
  onFixed: (value: string) => void;
}) {
  const active = group.axes.includes(axis);
  const range = group.axisRanges[axis];
  const fixed = axis === "x" ? group.fixedXNt : axis === "y" ? group.fixedYNt : group.fixedZNt;
  if (!active) {
    return (
      <td style={tdStyle}>
        <label style={{ display: "grid", gap: 4 }}>
          固定 {axis.toUpperCase()} nT
          <input value={fixed} onChange={(event) => onFixed(event.target.value)} style={{ ...inputStyle, minWidth: 90 }} />
        </label>
      </td>
    );
  }
  return (
    <td style={tdStyle}>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 76px)", gap: 4 }}>
        <input title={`${axis} start`} value={range.startNt} onChange={(event) => onRange({ startNt: event.target.value })} style={inputStyle} />
        <input title={`${axis} stop`} value={range.stopNt} onChange={(event) => onRange({ stopNt: event.target.value })} style={inputStyle} />
        <input title={`${axis} step`} value={range.stepNt} onChange={(event) => onRange({ stepNt: event.target.value })} style={inputStyle} />
      </div>
      <div style={{ marginTop: 4, color: "var(--color-text-muted)" }}>start / stop / step nT</div>
    </td>
  );
}

function FieldEditor({ rows, setRows }: { rows: StepDraftRow[]; setRows: React.Dispatch<React.SetStateAction<StepDraftRow[]>> }) {
  const first = rows[0] ?? defaultRow();
  const updateAll = (key: "bxNt" | "byNt" | "bzNt", value: string) => setRows((current) => current.map((row) => ({ ...row, [key]: value })));
  return (
    <div>
      <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>磁场参数包</h2>
      <p style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)" }}>主变量是 Bx/By/Bz；三轴电流换算在设备工作台根据 runtime zero 和 coil constant 预览。</p>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(3, minmax(140px, 1fr))", gap: "var(--space-3)" }}>
        <LabelInput label="Bx nT" value={first.bxNt} onChange={(value) => updateAll("bxNt", value)} />
        <LabelInput label="By nT" value={first.byNt} onChange={(value) => updateAll("byNt", value)} />
        <LabelInput label="Bz nT" value={first.bzNt} onChange={(value) => updateAll("bzNt", value)} />
      </div>
    </div>
  );
}

function SmbTemplateEditor({ rows, setRows }: { rows: StepDraftRow[]; setRows: React.Dispatch<React.SetStateAction<StepDraftRow[]>> }) {
  const first = rows[0] ?? defaultRow();
  const updateAll = (patch: Partial<StepDraftRow>) => setRows((current) => current.map((row) => ({ ...row, ...patch })));
  return (
    <div>
      <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>SMB100A RF Sweep 谱线模板</h2>
      <p style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)" }}>频率、步长和 sweep output 电压统一写入 JSON 的 SI 值；RF output 不在实验计划中直接打开。</p>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "var(--space-3)" }}>
        <UnitInput label="RF 起始频率" value={first.rfStart} unit={first.rfStartUnit} units={frequencyUnits} onValue={(value) => updateAll({ rfStart: value })} onUnit={(unit) => updateAll({ rfStartUnit: unit as FrequencyUnit })} />
        <UnitInput label="RF 截止频率" value={first.rfStop} unit={first.rfStopUnit} units={frequencyUnits} onValue={(value) => updateAll({ rfStop: value })} onUnit={(unit) => updateAll({ rfStopUnit: unit as FrequencyUnit })} />
        <UnitInput label="RF 频率步进" value={first.rfStep} unit={first.rfStepUnit} units={frequencyUnits} onValue={(value) => updateAll({ rfStep: value })} onUnit={(unit) => updateAll({ rfStepUnit: unit as FrequencyUnit })} />
        <LabelInput label="每频点 dwell ms" value={first.dwellMs} onChange={(value) => updateAll({ dwellMs: value })} />
        <LabelInput label="功率电平 dBm" value={first.powerDbm} onChange={(value) => updateAll({ powerDbm: value })} />
        <UnitInput label="Sweep output start" value={first.sweepOutputStart} unit={first.sweepOutputStartUnit} units={voltageUnits} onValue={(value) => updateAll({ sweepOutputStart: value })} onUnit={(unit) => updateAll({ sweepOutputStartUnit: unit as VoltageUnit })} />
        <UnitInput label="Sweep output stop" value={first.sweepOutputStop} unit={first.sweepOutputStopUnit} units={voltageUnits} onValue={(value) => updateAll({ sweepOutputStop: value })} onUnit={(unit) => updateAll({ sweepOutputStopUnit: unit as VoltageUnit })} />
      </div>
    </div>
  );
}

function OeTemplateEditor({ rows, setRows }: { rows: StepDraftRow[]; setRows: React.Dispatch<React.SetStateAction<StepDraftRow[]>> }) {
  const first = rows[0] ?? defaultRow();
  const updateAll = (patch: Partial<StepDraftRow>) => setRows((current) => current.map((row) => ({ ...row, ...patch })));
  return (
    <div>
      <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>OE1022D 采集窗口模板</h2>
      <p style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)" }}>
        采集模式为 follow_rf_sweep：step 开始前提前采集，RF sweep 结束后延后停止。这里只保留四个灵活参数，其余实时配置放设备工作台。
      </p>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(190px, 1fr))", gap: "var(--space-3)", marginBottom: "var(--space-4)" }}>
        <LabelInput label="提前开始 pre_start_ms" value={first.oePreStartMs} onChange={(value) => updateAll({ oePreStartMs: value })} />
        <LabelInput label="延后停止 post_stop_ms" value={first.oePostStopMs} onChange={(value) => updateAll({ oePostStopMs: value })} />
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "var(--space-3)" }}>
        <OeChannelEditor title="Ch-A" row={first} prefix="chA" onPatch={updateAll} />
        <OeChannelEditor title="Ch-B" row={first} prefix="chB" onPatch={updateAll} />
      </div>
      <details style={{ marginTop: "var(--space-4)" }}>
        <summary style={{ cursor: "pointer", fontWeight: 600 }}>灰显字段说明</summary>
        <div style={{ display: "grid", gap: 6, marginTop: "var(--space-2)", color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)" }}>
          <div>Ch-B 外部参考时，内部频率、内部扫频类型、扫频起止/步进/运行模式显示但禁用。</div>
          <div>正弦输出固定幅值时，扫幅开始/截止/步进/时间、直流输出幅值显示但禁用。</div>
          <div>Channel source 为 A-R 时，CH1/CH2 AUXOUT 电压显示但禁用。</div>
        </div>
      </details>
    </div>
  );
}

function OeChannelEditor({
  title,
  row,
  prefix,
  onPatch,
}: {
  title: string;
  row: StepDraftRow;
  prefix: "chA" | "chB";
  onPatch: (patch: Partial<StepDraftRow>) => void;
}) {
  const keys = {
    tc: `${prefix}TimeConstantS`,
    slope: `${prefix}FilterSlope`,
    reserve: `${prefix}DynamicReserve`,
    sensitivity: `${prefix}Sensitivity`,
  } as const;
  return (
    <div style={{ border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", padding: "var(--space-3)" }}>
      <h3 style={{ fontSize: "var(--font-size-md)", marginBottom: "var(--space-2)" }}>{title}</h3>
      <div style={{ display: "grid", gap: "var(--space-2)" }}>
        <LabelInput label="滤波器时间常数 s" value={String(row[keys.tc])} onChange={(value) => onPatch({ [keys.tc]: value })} />
        <SelectInput label="滤波器陡降 dB/oct" value={String(row[keys.slope])} options={slopeOptions} onChange={(value) => onPatch({ [keys.slope]: value })} />
        <SelectInput label="动态储备" value={String(row[keys.reserve])} options={reserveOptions} onChange={(value) => onPatch({ [keys.reserve]: value })} />
        <SelectInput label="灵敏度" value={String(row[keys.sensitivity])} options={sensitivityOptions} onChange={(value) => onPatch({ [keys.sensitivity]: value })} />
      </div>
    </div>
  );
}

function LaserEditor({ rows, setRows }: { rows: StepDraftRow[]; setRows: React.Dispatch<React.SetStateAction<StepDraftRow[]>> }) {
  const first = rows[0] ?? defaultRow();
  const updateAll = (patch: Partial<StepDraftRow>) => setRows((current) => current.map((row) => ({ ...row, ...patch })));
  return (
    <div>
      <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>激光固定功率模板</h2>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(160px, 1fr))", gap: "var(--space-3)" }}>
        <LabelInput label="功率 mW" value={first.laserPowerMw} onChange={(value) => updateAll({ laserPowerMw: value })} />
        <label style={{ display: "flex", gap: 8, alignItems: "center", fontSize: "var(--font-size-sm)", marginTop: 22 }}>
          <input type="checkbox" checked={first.laserEnabled} onChange={(event) => updateAll({ laserEnabled: event.target.checked })} />
          模板中启用激光
        </label>
      </div>
    </div>
  );
}

function StepEditor({
  rows,
  setRows,
  onAdd,
  onDuplicate,
  onDelete,
  projection,
  resolved,
}: {
  rows: StepDraftRow[];
  setRows: React.Dispatch<React.SetStateAction<StepDraftRow[]>>;
  onAdd: () => void;
  onDuplicate: (index: number) => void;
  onDelete: (index: number) => void;
  projection: ExperimentPlanProjection | null;
  resolved: ResolvedPlanPreview | null;
}) {
  const updateRow = (index: number, patch: Partial<StepDraftRow>) => setRows((current) => current.map((row, i) => i === index ? { ...row, ...patch } : row));
  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-3)" }}>
        <div>
          <h2 style={{ fontSize: "var(--font-size-lg)" }}>Step 表编辑器</h2>
          <div style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)" }}>每行是一条 ODMR 谱线；RF sweep 内部频点不直接变成 Step。</div>
        </div>
        <button onClick={onAdd} style={btnPrimary}>新增 Step</button>
      </div>
      <div style={{ overflowX: "auto" }}>
        <table style={{ ...tableStyle, minWidth: 2560 }}>
          <thead>
            <tr>
              <th style={{ ...thStyle, position: "sticky", left: 0, background: "var(--color-surface)", zIndex: 1 }}>Step</th>
              <th style={thStyle}>扫描组</th>
              <th style={thStyle}>Bx</th>
              <th style={thStyle}>By</th>
              <th style={thStyle}>Bz</th>
              <th style={thStyle}>RF start</th>
              <th style={thStyle}>RF stop</th>
              <th style={thStyle}>RF step</th>
              <th style={thStyle}>Dwell ms</th>
              <th style={thStyle}>Power dBm</th>
              <th style={thStyle}>Sweep out start V</th>
              <th style={thStyle}>Sweep out stop V</th>
              <th style={thStyle}>Laser mW</th>
              <th style={thStyle}>Laser enable</th>
              <th style={thStyle}>OE pre</th>
              <th style={thStyle}>OE post</th>
              <th style={thStyle}>Ch-A TC/Slope</th>
              <th style={thStyle}>Ch-B TC/Slope</th>
              <th style={thStyle}>操作</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row, index) => (
              <tr key={row.id}>
                <td style={{ ...tdStyle, position: "sticky", left: 0, background: "var(--color-surface)", fontWeight: 600 }}>
                  <input value={row.id} onChange={(event) => updateRow(index, { id: event.target.value })} style={{ ...inputStyle, minWidth: 130 }} />
                </td>
                <td style={tdStyle}>{row.groupId ?? "手动"}</td>
                <StepText value={row.bxNt} onChange={(value) => updateRow(index, { bxNt: value })} />
                <StepText value={row.byNt} onChange={(value) => updateRow(index, { byNt: value })} />
                <StepText value={row.bzNt} onChange={(value) => updateRow(index, { bzNt: value })} />
                <StepUnit value={row.rfStart} unit={row.rfStartUnit} units={frequencyUnits} onValue={(value) => updateRow(index, { rfStart: value })} onUnit={(unit) => updateRow(index, { rfStartUnit: unit as FrequencyUnit })} />
                <StepUnit value={row.rfStop} unit={row.rfStopUnit} units={frequencyUnits} onValue={(value) => updateRow(index, { rfStop: value })} onUnit={(unit) => updateRow(index, { rfStopUnit: unit as FrequencyUnit })} />
                <StepUnit value={row.rfStep} unit={row.rfStepUnit} units={frequencyUnits} onValue={(value) => updateRow(index, { rfStep: value })} onUnit={(unit) => updateRow(index, { rfStepUnit: unit as FrequencyUnit })} />
                <StepText value={row.dwellMs} onChange={(value) => updateRow(index, { dwellMs: value })} />
                <StepText value={row.powerDbm} onChange={(value) => updateRow(index, { powerDbm: value })} />
                <StepUnit value={row.sweepOutputStart} unit={row.sweepOutputStartUnit} units={voltageUnits} onValue={(value) => updateRow(index, { sweepOutputStart: value })} onUnit={(unit) => updateRow(index, { sweepOutputStartUnit: unit as VoltageUnit })} />
                <StepUnit value={row.sweepOutputStop} unit={row.sweepOutputStopUnit} units={voltageUnits} onValue={(value) => updateRow(index, { sweepOutputStop: value })} onUnit={(unit) => updateRow(index, { sweepOutputStopUnit: unit as VoltageUnit })} />
                <StepText value={row.laserPowerMw} onChange={(value) => updateRow(index, { laserPowerMw: value })} />
                <td style={tdStyle}><input type="checkbox" checked={row.laserEnabled} onChange={(event) => updateRow(index, { laserEnabled: event.target.checked })} /></td>
                <StepText value={row.oePreStartMs} onChange={(value) => updateRow(index, { oePreStartMs: value })} />
                <StepText value={row.oePostStopMs} onChange={(value) => updateRow(index, { oePostStopMs: value })} />
                <td style={tdStyle}>{row.chATimeConstantS}s / {row.chAFilterSlope} dB/oct</td>
                <td style={tdStyle}>{row.chBTimeConstantS}s / {row.chBFilterSlope} dB/oct</td>
                <td style={tdStyle}>
                  <button onClick={() => onDuplicate(index)} style={btnSecondary}>复制</button>{" "}
                  <button onClick={() => onDelete(index)} style={btnDanger}>删除</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <ProjectedStepTable projection={projection} resolved={resolved} />
    </div>
  );
}

function ProjectedStepTable({ projection, resolved }: { projection: ExperimentPlanProjection | null; resolved: ResolvedPlanPreview | null }) {
  if (!projection) return null;
  return (
    <div style={{ marginTop: "var(--space-4)", overflowX: "auto" }}>
      <h3 style={{ fontSize: "var(--font-size-md)", marginBottom: "var(--space-2)" }}>Step 投影预览</h3>
      <table style={{ ...tableStyle, minWidth: 1500 }}>
        <thead>
          <tr>
            <th style={thStyle}>Step</th>
            <th style={thStyle}>扫描组</th>
            <th style={thStyle}>B 向量 nT</th>
            <th style={thStyle}>RF sweep</th>
            <th style={thStyle}>功率</th>
            <th style={thStyle}>Sweep output</th>
            <th style={thStyle}>Laser</th>
            <th style={thStyle}>OE1022D</th>
            <th style={thStyle}>耗时</th>
            <th style={thStyle}>状态</th>
          </tr>
        </thead>
        <tbody>
          {projection.step_rows.map((step) => (
            <tr key={`${step.step_index}.${step.step_id}`}>
              <td style={tdStyle}>{step.step_id}</td>
              <td style={tdStyle}>{step.group_id ?? "手动"}</td>
              <td style={tdStyle}>[{step.bx_nt}, {step.by_nt}, {step.bz_nt}]</td>
              <td style={tdStyle}>{fmtHz(step.rf_start_hz ?? step.smb100a_frequency_hz)} → {fmtHz(step.rf_stop_hz)} / step {step.rf_step_hz ?? "—"} Hz / dwell {step.dwell_ms ?? "—"} ms</td>
              <td style={tdStyle}>{step.smb100a_power_dbm == null ? "—" : `${step.smb100a_power_dbm} dBm`}</td>
              <td style={tdStyle}>{step.smb100a_sweep_output_start_v ?? "—"} V → {step.smb100a_sweep_output_stop_v ?? "—"} V</td>
              <td style={tdStyle}><span style={badge(step.laser_enabled ? "on" : "off")}>{step.laser_enabled ? "ON" : "OFF"}</span> {step.laser_power_mw == null ? "" : `${step.laser_power_mw} mW`}</td>
              <td style={tdStyle}>pre {step.oe_pre_start_ms ?? "—"} ms / post {step.oe_post_stop_ms ?? "—"} ms · Ch-A {step.oe_ch_a_time_constant_s ?? "—"}s {step.oe_ch_a_filter_slope_db_oct ?? "—"}dB · Ch-B {step.oe_ch_b_time_constant_s ?? "—"}s {step.oe_ch_b_filter_slope_db_oct ?? "—"}dB</td>
              <td style={tdStyle}>{step.estimated_duration_s == null ? "—" : `${step.estimated_duration_s.toFixed(2)} s`}</td>
              <td style={tdStyle}><span style={badge(step.executable && (resolved?.executable ?? true) ? "ok" : "blocked")}>{step.executable ? "预览可用" : "阻塞"}</span></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function DeviceParameterTables({ projection, resolved }: { projection: ExperimentPlanProjection | null; resolved: ResolvedPlanPreview | null }) {
  if (!projection) {
    return (
      <div style={cardStyle}>
        <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>设备参数表</h2>
        <div style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-sm)" }}>导入实验 JSON 或应用草稿后，这里会显示 Step 和组合预览。</div>
      </div>
    );
  }
  return (
    <div style={{ display: "grid", gap: "var(--space-4)" }}>
      <div style={cardStyle}>
        <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>设备参数表</h2>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "var(--space-3)", fontSize: "var(--font-size-xs)" }}>
          <div><strong>主变量:</strong> 磁场</div>
          <div><strong>第二变量:</strong> SMB100A</div>
          <div><strong>第三变量:</strong> 激光</div>
          <div><strong>第四变量:</strong> OE1022D</div>
        </div>
        {projection.warnings.length > 0 && <div style={{ marginTop: "var(--space-2)", color: "var(--color-warning)", fontSize: "var(--font-size-xs)" }}>{projection.warnings.join(" · ")}</div>}
      </div>
      <ProjectedStepTable projection={projection} resolved={resolved} />
      <div style={cardStyle}>
        <h3 style={{ fontSize: "var(--font-size-md)", marginBottom: "var(--space-1)" }}>ODMR 谱线图</h3>
        <p style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)", margin: 0 }}>
          暂无真实谱线。完成一次真实采集后，从运行产物的 <code>.rall + index.jsonl</code> 解析并显示 RF frequency vs OE 信号；计划编辑阶段不再生成模拟谱线。
        </p>
      </div>
    </div>
  );
}

function JsonView({ value }: { value: unknown }) {
  return <pre style={monoBlock}>{JSON.stringify(value, null, 2)}</pre>;
}

function LabelInput({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <label style={{ display: "grid", gap: 5, fontSize: "var(--font-size-xs)" }}>
      {label}
      <input value={value} onChange={(event) => onChange(event.target.value)} style={inputStyle} />
    </label>
  );
}

function SelectInput({ label, value, options, onChange }: { label: string; value: string; options: string[]; onChange: (value: string) => void }) {
  return (
    <label style={{ display: "grid", gap: 5, fontSize: "var(--font-size-xs)" }}>
      {label}
      <select value={value} onChange={(event) => onChange(event.target.value)} style={inputStyle}>
        {options.map((option) => <option key={option} value={option}>{option}</option>)}
      </select>
    </label>
  );
}

function UnitInput({
  label,
  value,
  unit,
  units,
  onValue,
  onUnit,
}: {
  label: string;
  value: string;
  unit: string;
  units: string[];
  onValue: (value: string) => void;
  onUnit: (unit: string) => void;
}) {
  return (
    <label style={{ display: "grid", gap: 5, fontSize: "var(--font-size-xs)" }}>
      {label}
      <span style={{ display: "grid", gridTemplateColumns: "1fr 82px", gap: 6 }}>
        <input value={value} onChange={(event) => onValue(event.target.value)} style={inputStyle} />
        <select value={unit} onChange={(event) => onUnit(event.target.value)} style={inputStyle}>
          {units.map((option) => <option key={option} value={option}>{option}</option>)}
        </select>
      </span>
    </label>
  );
}

function StepText({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return <td style={tdStyle}><input value={value} onChange={(event) => onChange(event.target.value)} style={{ ...inputStyle, minWidth: 92 }} /></td>;
}

function StepUnit({
  value,
  unit,
  units,
  onValue,
  onUnit,
}: {
  value: string;
  unit: string;
  units: string[];
  onValue: (value: string) => void;
  onUnit: (unit: string) => void;
}) {
  return (
    <td style={tdStyle}>
      <div style={{ display: "grid", gridTemplateColumns: "96px 76px", gap: 4 }}>
        <input value={value} onChange={(event) => onValue(event.target.value)} style={inputStyle} />
        <select value={unit} onChange={(event) => onUnit(event.target.value)} style={inputStyle}>
          {units.map((option) => <option key={option} value={option}>{option}</option>)}
        </select>
      </div>
    </td>
  );
}

function groupBy<T>(values: T[], keyFn: (value: T) => string) {
  return values.reduce<Record<string, T[]>>((acc, value) => {
    const key = keyFn(value);
    acc[key] = acc[key] ?? [];
    acc[key].push(value);
    return acc;
  }, {});
}

function rfFromRow(row: StepDraftRow) {
  return {
    start_hz: toHz(row.rfStart, row.rfStartUnit),
    stop_hz: toHz(row.rfStop, row.rfStopUnit),
    step_hz: toHz(row.rfStep, row.rfStepUnit),
    dwell_ms: parseNumber(row.dwellMs, 300),
    power_dbm: optionalNumber(row.powerDbm),
    spacing: "LINear",
    shape: "SAWtooth",
    sweep_output_start_v: toVolt(row.sweepOutputStart, row.sweepOutputStartUnit),
    sweep_output_stop_v: toVolt(row.sweepOutputStop, row.sweepOutputStopUnit),
  };
}

function groupPointCount(group: MagneticScanGroupDraft) {
  if (!group.enabled) return 0;
  return group.axes.reduce((product, axis) => product * axisValues(group, axis).length, 1);
}

function groupPointFormula(group: MagneticScanGroupDraft) {
  const terms = group.axes.map((axis) => `${axis.toUpperCase()}=${axisValues(group, axis).length}`);
  return terms.length > 0 ? `${terms.join(" × ")} = ${groupPointCount(group)}` : "0";
}

function axisValues(group: MagneticScanGroupDraft, axis: MagneticAxis) {
  const range = group.axisRanges[axis];
  const start = parseNumber(range.startNt, 0);
  const stop = parseNumber(range.stopNt, start);
  const step = Math.abs(parseNumber(range.stepNt, 0));
  if (step <= 0) return [];
  const values: number[] = [];
  const ascending = stop >= start;
  let value = start;
  while (true) {
    values.push(value);
    value = ascending ? value + step : value - step;
    if (ascending ? value > stop + Number.EPSILON : value < stop - Number.EPSILON) break;
  }
  return values;
}

function rowsFromScanGroups(groups: MagneticScanGroupDraft[], template: StepDraftRow, limit = 200): StepDraftRow[] {
  const out: StepDraftRow[] = [];
  const ordered = [...groups].filter((group) => group.enabled).sort((a, b) => a.order - b.order);
  for (const group of ordered) {
    let localIndex = 0;
    const xs = group.axes.includes("x") ? axisValues(group, "x") : [parseNumber(group.fixedXNt)];
    const ys = group.axes.includes("y") ? axisValues(group, "y") : [parseNumber(group.fixedYNt)];
    const zs = group.axes.includes("z") ? axisValues(group, "z") : [parseNumber(group.fixedZNt)];
    for (const bx of xs) {
      for (const by of ys) {
        for (const bz of zs) {
          if (out.length >= limit) return out;
          out.push({
            ...template,
            id: `${group.groupId}_${String(localIndex).padStart(6, "0")}`,
            groupId: group.groupId,
            bxNt: String(bx),
            byNt: String(by),
            bzNt: String(bz),
            ...rowPatchFromGroupOverride(group),
          });
          localIndex += 1;
        }
      }
    }
  }
  return out;
}

function rowPatchFromGroupOverride(group: MagneticScanGroupDraft): Partial<StepDraftRow> {
  if (!group.overrideEnabled) return {};
  return {
    ...(group.powerDbm ? { powerDbm: group.powerDbm } : {}),
    ...(group.laserPowerMw ? { laserPowerMw: group.laserPowerMw } : {}),
    laserEnabled: group.laserEnabled,
    ...(group.oePreStartMs ? { oePreStartMs: group.oePreStartMs } : {}),
    ...(group.oePostStopMs ? { oePostStopMs: group.oePostStopMs } : {}),
    ...(group.chATimeConstantS ? { chATimeConstantS: group.chATimeConstantS } : {}),
    ...(group.chAFilterSlope ? { chAFilterSlope: group.chAFilterSlope } : {}),
    ...(group.chADynamicReserve ? { chADynamicReserve: group.chADynamicReserve } : {}),
    ...(group.chASensitivity ? { chASensitivity: group.chASensitivity } : {}),
    ...(group.chBTimeConstantS ? { chBTimeConstantS: group.chBTimeConstantS } : {}),
    ...(group.chBFilterSlope ? { chBFilterSlope: group.chBFilterSlope } : {}),
    ...(group.chBDynamicReserve ? { chBDynamicReserve: group.chBDynamicReserve } : {}),
    ...(group.chBSensitivity ? { chBSensitivity: group.chBSensitivity } : {}),
  };
}

function fieldSpaceFromScanGroups(groups: MagneticScanGroupDraft[]) {
  return {
    mode: "grouped_grid_scan",
    unit: "nT",
    groups: groups.map((group) => ({
      group_id: group.groupId,
      label_cn: group.labelCn,
      scan_kind: group.scanKind,
      axes: group.axes,
      axis_ranges_nt: {
        x: {
          start: parseNumber(group.axisRanges.x.startNt, 0),
          stop: parseNumber(group.axisRanges.x.stopNt, 0),
          step: Math.abs(parseNumber(group.axisRanges.x.stepNt, 1)),
        },
        y: {
          start: parseNumber(group.axisRanges.y.startNt, 0),
          stop: parseNumber(group.axisRanges.y.stopNt, 0),
          step: Math.abs(parseNumber(group.axisRanges.y.stepNt, 1)),
        },
        z: {
          start: parseNumber(group.axisRanges.z.startNt, 0),
          stop: parseNumber(group.axisRanges.z.stopNt, 0),
          step: Math.abs(parseNumber(group.axisRanges.z.stepNt, 1)),
        },
      },
      fixed_axes_nt: {
        x: parseNumber(group.fixedXNt, 0),
        y: parseNumber(group.fixedYNt, 0),
        z: parseNumber(group.fixedZNt, 0),
      },
      order: group.order,
      enabled: group.enabled,
    })),
  };
}

function groupOverridesFromScanGroups(groups: MagneticScanGroupDraft[]) {
  const overrides: PlanRecord = {};
  for (const group of groups) {
    if (!group.overrideEnabled) continue;
    const groupOverride: PlanRecord = {};
    const rf: PlanRecord = {};
    const power = optionalNumber(group.powerDbm);
    if (power !== undefined) rf.power_dbm = power;
    if (Object.keys(rf).length > 0) groupOverride.rf_sweep = rf;
    const laserPower = optionalNumber(group.laserPowerMw);
    if (laserPower !== undefined || group.laserEnabled) {
      groupOverride.laser = {
        ...(laserPower !== undefined ? { power_mw: laserPower } : {}),
        enabled: group.laserEnabled,
      };
    }
    const oePatch = oeFromRow({
      ...defaultRow(),
      oePreStartMs: group.oePreStartMs || defaultRow().oePreStartMs,
      oePostStopMs: group.oePostStopMs || defaultRow().oePostStopMs,
      chATimeConstantS: group.chATimeConstantS,
      chAFilterSlope: group.chAFilterSlope,
      chADynamicReserve: group.chADynamicReserve,
      chASensitivity: group.chASensitivity,
      chBTimeConstantS: group.chBTimeConstantS,
      chBFilterSlope: group.chBFilterSlope,
      chBDynamicReserve: group.chBDynamicReserve,
      chBSensitivity: group.chBSensitivity,
    });
    const oe = compactOeOverride(oePatch, group);
    if (Object.keys(oe).length > 0) groupOverride.oe1022d_acquisition = oe;
    if (Object.keys(groupOverride).length > 0) overrides[group.groupId] = groupOverride;
  }
  return overrides;
}

function compactOeOverride(oe: ReturnType<typeof oeFromRow>, group: MagneticScanGroupDraft): PlanRecord {
  const out: PlanRecord = {};
  if (group.oePreStartMs) out.pre_start_ms = oe.pre_start_ms;
  if (group.oePostStopMs) out.post_stop_ms = oe.post_stop_ms;
  const channels: PlanRecord = {};
  const chA: PlanRecord = {};
  const chB: PlanRecord = {};
  if (group.chATimeConstantS) chA.time_constant_s = oe.channels.ch_a.time_constant_s;
  if (group.chAFilterSlope) chA.filter_slope_db_oct = oe.channels.ch_a.filter_slope_db_oct;
  if (group.chADynamicReserve) chA.dynamic_reserve = oe.channels.ch_a.dynamic_reserve;
  if (group.chASensitivity) chA.sensitivity = oe.channels.ch_a.sensitivity;
  if (group.chBTimeConstantS) chB.time_constant_s = oe.channels.ch_b.time_constant_s;
  if (group.chBFilterSlope) chB.filter_slope_db_oct = oe.channels.ch_b.filter_slope_db_oct;
  if (group.chBDynamicReserve) chB.dynamic_reserve = oe.channels.ch_b.dynamic_reserve;
  if (group.chBSensitivity) chB.sensitivity = oe.channels.ch_b.sensitivity;
  if (Object.keys(chA).length > 0) channels.ch_a = chA;
  if (Object.keys(chB).length > 0) channels.ch_b = chB;
  if (Object.keys(channels).length > 0) out.channels = channels;
  return out;
}

function laserFromRow(row: StepDraftRow) {
  return {
    mode: "fixed_power",
    power_mw: optionalNumber(row.laserPowerMw),
    enabled: row.laserEnabled,
  };
}

function oeFromRow(row: StepDraftRow) {
  return {
    mode: "follow_rf_sweep",
    pre_start_ms: parseNumber(row.oePreStartMs, 50),
    post_stop_ms: parseNumber(row.oePostStopMs, 50),
    channels: {
      ch_a: {
        time_constant_s: optionalNumber(row.chATimeConstantS),
        filter_slope_db_oct: optionalNumber(row.chAFilterSlope),
        dynamic_reserve: row.chADynamicReserve,
        sensitivity: row.chASensitivity,
      },
      ch_b: {
        time_constant_s: optionalNumber(row.chBTimeConstantS),
        filter_slope_db_oct: optionalNumber(row.chBFilterSlope),
        dynamic_reserve: row.chBDynamicReserve,
        sensitivity: row.chBSensitivity,
      },
    },
  };
}

function buildDraftJson(base: PlanRecord | null, rows: StepDraftRow[], scanGroups?: MagneticScanGroupDraft[]) {
  const next = structuredClone(base ?? {
    schema_version: "0.1.0",
    kind: "experiment_plan",
    id: `table_draft_${new Date().toISOString().replace(/[:.]/g, "_")}`,
  }) as PlanRecord;
  const safeRows = rows.length > 0 ? rows : [defaultRow()];
  const first = safeRows[0];
  const enabledScanGroups = (scanGroups ?? []).filter((group) => group.enabled);
  if (enabledScanGroups.length > 0) {
    next.field_space = fieldSpaceFromScanGroups(scanGroups ?? []);
    next.group_overrides = groupOverridesFromScanGroups(scanGroups ?? []);
    delete next.steps;
  } else {
    next.field_space = {
      mode: "explicit_points",
      unit: "nT",
      points: safeRows.map((row) => [parseNumber(row.bxNt), parseNumber(row.byNt), parseNumber(row.bzNt)]),
    };
    next.steps = safeRows.map((row, index) => ({
      step_id: row.id || `spectrum_${index.toString().padStart(4, "0")}`,
      group_id: row.groupId,
      b_target_nt: [parseNumber(row.bxNt), parseNumber(row.byNt), parseNumber(row.bzNt)],
      ...(rowDiffersForRf(row, first) ? { rf_sweep: rfFromRow(row) } : {}),
      ...(rowDiffersForLaser(row, first) ? { laser: laserFromRow(row) } : {}),
      ...(rowDiffersForOe(row, first) ? { oe1022d_acquisition: oeFromRow(row) } : {}),
    }));
    delete next.group_overrides;
  }
  next.spectrum_template = {
    ...asRecord(next.spectrum_template),
    rf_sweep: rfFromRow(first),
    laser: laserFromRow(first),
    oe1022d_acquisition: oeFromRow(first),
  };
  delete next.manual_steps;
  next.runtime_requirements = {
    ...asRecord(next.runtime_requirements),
    require_preflight: true,
    require_zero_lock: true,
  };
  return next;
}

function rowDiffersForRf(row: StepDraftRow, first: StepDraftRow) {
  return row.rfStart !== first.rfStart || row.rfStartUnit !== first.rfStartUnit || row.rfStop !== first.rfStop || row.rfStopUnit !== first.rfStopUnit || row.rfStep !== first.rfStep || row.rfStepUnit !== first.rfStepUnit || row.dwellMs !== first.dwellMs || row.powerDbm !== first.powerDbm || row.sweepOutputStart !== first.sweepOutputStart || row.sweepOutputStop !== first.sweepOutputStop;
}

function rowDiffersForLaser(row: StepDraftRow, first: StepDraftRow) {
  return row.laserPowerMw !== first.laserPowerMw || row.laserEnabled !== first.laserEnabled;
}

function rowDiffersForOe(row: StepDraftRow, first: StepDraftRow) {
  return row.oePreStartMs !== first.oePreStartMs || row.oePostStopMs !== first.oePostStopMs || row.chATimeConstantS !== first.chATimeConstantS || row.chAFilterSlope !== first.chAFilterSlope || row.chADynamicReserve !== first.chADynamicReserve || row.chASensitivity !== first.chASensitivity || row.chBTimeConstantS !== first.chBTimeConstantS || row.chBFilterSlope !== first.chBFilterSlope || row.chBDynamicReserve !== first.chBDynamicReserve || row.chBSensitivity !== first.chBSensitivity;
}

function rowsFromDraft(plan: PlanRecord) {
  const scanGroups = scanGroupsFromDraft(plan);
  const mode = asRecord(plan.field_space).mode;
  const grouped = mode === "grouped_grid_scan" || mode === "grouped_path_scan";
  const template = asRecord(plan.spectrum_template);
  const rf = asRecord(template.rf_sweep ?? template.rf);
  const laser = asRecord(template.laser);
  const oe = asRecord(template.oe1022d_acquisition ?? template.oe1022d);
  if (grouped) {
    const templateRow = rowFromPlanParts([0, 0, 0], rf, laser, oe, "spectrum_template");
    return rowsFromScanGroups(scanGroups, templateRow, 200);
  }
  const pointsFromSteps = getArray(plan, "steps").map((step) => readBTarget(step));
  const points = pointsFromSteps.length > 0 ? pointsFromSteps : getArray(asRecord(plan.field_space), "points").map(readVector);
  const safePoints = points.length > 0 ? points : [[0, 0, 0]];
  const steps = getArray(plan, "steps");
  return safePoints.map((point, index) => {
    const step = isRecord(steps[index]) ? steps[index] : {};
    const stepRf = asRecord(step.rf_sweep ?? step.smb100a);
    const stepLaser = asRecord(step.laser);
    const stepOe = asRecord(step.oe1022d_acquisition ?? step.oe1022d);
    const mergedRf = { ...rf, ...stepRf };
    const mergedLaser = { ...laser, ...stepLaser };
    const mergedOe = deepMerge(oe, stepOe);
    return rowFromPlanParts(point, mergedRf, mergedLaser, mergedOe, String(step.step_id ?? `spectrum_${index.toString().padStart(4, "0")}`));
  });
}

function scanGroupsFromDraft(plan: PlanRecord): MagneticScanGroupDraft[] {
  const defaults = defaultScanGroups();
  const fieldSpace = asRecord(plan.field_space);
  if (fieldSpace.mode !== "grouped_grid_scan" && fieldSpace.mode !== "grouped_path_scan") return defaults;
  const groups = getArray(fieldSpace, "groups");
  if (groups.length === 0) return defaults;
  const overrides = asRecord(plan.group_overrides);
  return groups.map((value, index) => {
    const group = asRecord(value);
    const groupId = String(group.group_id ?? defaults[index]?.groupId ?? `group_${index}`);
    const range = asRecord(group.range_nt);
    const axisRanges = asRecord(group.axis_ranges_nt);
    const fixed = asRecord(group.fixed_axes_nt);
    const axesRaw = Array.isArray(group.axes) ? group.axes : [];
    const axes = axesRaw.filter((axis): axis is MagneticAxis => axis === "x" || axis === "y" || axis === "z");
    const groupOverride = asRecord(overrides[groupId]);
    const rf = asRecord(groupOverride.rf_sweep ?? groupOverride.rf);
    const laser = asRecord(groupOverride.laser);
    const oe = asRecord(groupOverride.oe1022d_acquisition ?? groupOverride.oe1022d);
    const channels = asRecord(oe.channels);
    const chA = asRecord(channels.ch_a);
    const chB = asRecord(channels.ch_b);
    return {
      groupId,
      labelCn: String(group.label_cn ?? defaults[index]?.labelCn ?? groupId),
      scanKind: readScanKind(group.scan_kind, axes.length),
      axes: axes.length > 0 ? axes : defaults[index]?.axes ?? ["x"],
      enabled: Boolean(group.enabled ?? true),
      axisRanges: {
        x: readAxisRangeDraft(axisRanges.x, range),
        y: readAxisRangeDraft(axisRanges.y, range),
        z: readAxisRangeDraft(axisRanges.z, range),
      },
      fixedXNt: String(fixed.x ?? fixed.bx_nt ?? "0"),
      fixedYNt: String(fixed.y ?? fixed.by_nt ?? "0"),
      fixedZNt: String(fixed.z ?? fixed.bz_nt ?? "0"),
      order: typeof group.order === "number" ? group.order : index,
      overrideEnabled: Object.keys(groupOverride).length > 0,
      powerDbm: String(rf.power_dbm ?? ""),
      laserPowerMw: String(laser.power_mw ?? ""),
      laserEnabled: Boolean(laser.enabled ?? false),
      oePreStartMs: String(oe.pre_start_ms ?? ""),
      oePostStopMs: String(oe.post_stop_ms ?? ""),
      chATimeConstantS: String(chA.time_constant_s ?? ""),
      chAFilterSlope: String(chA.filter_slope_db_oct ?? ""),
      chADynamicReserve: String(chA.dynamic_reserve ?? ""),
      chASensitivity: String(chA.sensitivity ?? ""),
      chBTimeConstantS: String(chB.time_constant_s ?? ""),
      chBFilterSlope: String(chB.filter_slope_db_oct ?? ""),
      chBDynamicReserve: String(chB.dynamic_reserve ?? ""),
      chBSensitivity: String(chB.sensitivity ?? ""),
    };
  });
}

function readScanKind(value: unknown, axisCount: number): MagneticScanKind {
  if (value === "line_1d" || value === "plane_2d" || value === "volume_3d" || value === "custom_grid") {
    return value;
  }
  if (axisCount === 1) return "line_1d";
  if (axisCount === 2) return "plane_2d";
  if (axisCount === 3) return "volume_3d";
  return "custom_grid";
}

function readAxisRangeDraft(value: unknown, fallback: PlanRecord): AxisRangeDraft {
  const range = asRecord(value);
  return {
    startNt: String(range.start ?? range.start_nt ?? fallback.start ?? fallback.start_nt ?? "0"),
    stopNt: String(range.stop ?? range.stop_nt ?? fallback.stop ?? fallback.stop_nt ?? "0"),
    stepNt: String(range.step ?? range.step_nt ?? fallback.step ?? fallback.step_nt ?? "1"),
  };
}

function rowFromPlanParts(point: number[], rf: PlanRecord, laser: PlanRecord, oe: PlanRecord, id: string): StepDraftRow {
  const row = defaultRow(id);
  const channels = asRecord(oe.channels);
  const chA = asRecord(channels.ch_a);
  const chB = asRecord(channels.ch_b);
  return {
    ...row,
    bxNt: String(point[0] ?? 0),
    byNt: String(point[1] ?? 0),
    bzNt: String(point[2] ?? 0),
    rfStart: fromHz(rf.start_hz ?? rf.frequency_hz, "GHz") || row.rfStart,
    rfStop: fromHz(rf.stop_hz, "GHz") || row.rfStop,
    rfStep: fromHz(rf.step_hz, "MHz") || row.rfStep,
    dwellMs: String(rf.dwell_ms ?? row.dwellMs),
    powerDbm: String(rf.power_dbm ?? row.powerDbm),
    sweepOutputStart: String(rf.sweep_output_start_v ?? row.sweepOutputStart),
    sweepOutputStop: String(rf.sweep_output_stop_v ?? row.sweepOutputStop),
    laserPowerMw: String(laser.power_mw ?? row.laserPowerMw),
    laserEnabled: Boolean(laser.enabled ?? row.laserEnabled),
    oePreStartMs: String(oe.pre_start_ms ?? row.oePreStartMs),
    oePostStopMs: String(oe.post_stop_ms ?? row.oePostStopMs),
    chATimeConstantS: String(chA.time_constant_s ?? oe.time_constant_s ?? row.chATimeConstantS),
    chAFilterSlope: String(chA.filter_slope_db_oct ?? oe.filter_slope_db_oct ?? row.chAFilterSlope),
    chADynamicReserve: String(chA.dynamic_reserve ?? row.chADynamicReserve),
    chASensitivity: String(chA.sensitivity ?? row.chASensitivity),
    chBTimeConstantS: String(chB.time_constant_s ?? oe.time_constant_s ?? row.chBTimeConstantS),
    chBFilterSlope: String(chB.filter_slope_db_oct ?? oe.filter_slope_db_oct ?? row.chBFilterSlope),
    chBDynamicReserve: String(chB.dynamic_reserve ?? row.chBDynamicReserve),
    chBSensitivity: String(chB.sensitivity ?? row.chBSensitivity),
  };
}

function readVector(value: unknown) {
  if (!Array.isArray(value)) return [0, 0, 0];
  return [Number(value[0] ?? 0), Number(value[1] ?? 0), Number(value[2] ?? 0)];
}

function readBTarget(value: unknown) {
  if (!isRecord(value)) return [0, 0, 0];
  return readVector(value.b_target_nt ?? value.b);
}

function deepMerge(base: PlanRecord, override: PlanRecord): PlanRecord {
  const out: PlanRecord = { ...base };
  for (const [key, value] of Object.entries(override)) {
    out[key] = isRecord(value) && isRecord(out[key]) ? deepMerge(out[key], value) : value;
  }
  return out;
}

function applyPackageToRows(rows: StepDraftRow[], device: string, values: PlanRecord) {
  return rows.map((row) => {
    if (device === "smb100a") {
      const rf = asRecord(values.rf_sweep);
      if (Object.keys(rf).length === 0) return row;
      return {
        ...row,
        rfStart: fromHz(rf.start_hz, "GHz") || row.rfStart,
        rfStop: fromHz(rf.stop_hz, "GHz") || row.rfStop,
        rfStep: fromHz(rf.step_hz, "MHz") || row.rfStep,
        dwellMs: String(rf.dwell_ms ?? row.dwellMs),
        powerDbm: String(rf.power_dbm ?? row.powerDbm),
        sweepOutputStart: String(rf.sweep_output_start_v ?? row.sweepOutputStart),
        sweepOutputStop: String(rf.sweep_output_stop_v ?? row.sweepOutputStop),
      };
    }
    if (device === "oe1022d") {
      const oe = asRecord(values.oe1022d_acquisition);
      const channels = asRecord(oe.channels);
      const chA = asRecord(channels.ch_a);
      const chB = asRecord(channels.ch_b);
      return {
        ...row,
        oePreStartMs: String(oe.pre_start_ms ?? row.oePreStartMs),
        oePostStopMs: String(oe.post_stop_ms ?? row.oePostStopMs),
        chATimeConstantS: String(chA.time_constant_s ?? row.chATimeConstantS),
        chAFilterSlope: String(chA.filter_slope_db_oct ?? row.chAFilterSlope),
        chADynamicReserve: String(chA.dynamic_reserve ?? row.chADynamicReserve),
        chASensitivity: String(chA.sensitivity ?? row.chASensitivity),
        chBTimeConstantS: String(chB.time_constant_s ?? row.chBTimeConstantS),
        chBFilterSlope: String(chB.filter_slope_db_oct ?? row.chBFilterSlope),
        chBDynamicReserve: String(chB.dynamic_reserve ?? row.chBDynamicReserve),
        chBSensitivity: String(chB.sensitivity ?? row.chBSensitivity),
      };
    }
    return row;
  });
}

function mergePackageIntoDraft(base: PlanRecord | null, pkg: DeviceDefaultPackage) {
  const next = structuredClone(base ?? {}) as PlanRecord;
  const devicePresets = asRecord(next.device_presets);
  devicePresets[pkg.device] = {
    package_id: pkg.package_id,
    source: pkg.source,
    risk_level: pkg.risk_level,
    values_si: pkg.values_si,
  };
  next.device_presets = devicePresets;
  return next;
}

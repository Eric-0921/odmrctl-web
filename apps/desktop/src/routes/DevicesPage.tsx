import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  Smb100aStatus,
  Oe1022dStatus,
  MagneticStatus,
  MagneticXyzPackageStatus,
  MagneticVectorApplyResult,
  LaserStatus,
  RuntimeZeroBaseline,
  WorkbenchSnapshot,
  StationPreflightReport,
  DeviceDiscoveryReport,
  DeviceProbeRequest,
  AutoBindReport,
  DeviceRoleRequest,
} from "../types/deviceWorkbench";

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const cardStyle: React.CSSProperties = {
  background: "var(--color-surface)",
  border: "1px solid var(--color-border)",
  borderRadius: "var(--radius-md)",
  padding: "var(--space-4)",
  boxShadow: "var(--shadow-card)",
};

const badgeOk: React.CSSProperties = {
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-sm)",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  background: "var(--color-success-soft)",
  color: "var(--color-success)",
};

const badgeFail: React.CSSProperties = {
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-sm)",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  background: "var(--color-danger-soft)",
  color: "var(--color-danger)",
};

const badgeNeutral: React.CSSProperties = {
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-sm)",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  background: "var(--color-disabled-bg)",
  color: "var(--color-disabled-text)",
};

const btnPrimary: React.CSSProperties = {
  padding: "6px 12px",
  borderRadius: "var(--radius-sm)",
  border: "none",
  background: "var(--color-primary)",
  color: "#fff",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  cursor: "pointer",
};

const btnSecondary: React.CSSProperties = {
  padding: "6px 12px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  color: "var(--color-text)",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  cursor: "pointer",
};

const btnDanger: React.CSSProperties = {
  padding: "6px 12px",
  borderRadius: "var(--radius-sm)",
  border: "none",
  background: "var(--color-danger)",
  color: "#fff",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  cursor: "pointer",
};

const btnDisabled: React.CSSProperties = {
  padding: "6px 12px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-disabled-bg)",
  color: "var(--color-disabled-text)",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  cursor: "not-allowed",
};

const inputStyle: React.CSSProperties = {
  padding: "4px 8px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  fontSize: "var(--font-size-xs)",
  flex: 1,
  fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
};

const smallMuted: React.CSSProperties = {
  fontSize: "var(--font-size-xs)",
  color: "var(--color-text-muted)",
};

const sectionTitle: React.CSSProperties = {
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  marginBottom: "var(--space-2)",
  color: "var(--color-text)",
};

const fmtA = (a?: number | null) => (a == null ? "—" : `${(a * 1000).toFixed(2)} mA`);
type WorkbenchTab = "station" | "magnetic" | "smb100a" | "oe1022d" | "laser";
type FrequencyUnit = "Hz" | "kHz" | "MHz" | "GHz";
type VoltageUnit = "V" | "mV";

const frequencyUnits: FrequencyUnit[] = ["Hz", "kHz", "MHz", "GHz"];
const voltageUnits: VoltageUnit[] = ["V", "mV"];
const hzFactor: Record<FrequencyUnit, number> = { Hz: 1, kHz: 1e3, MHz: 1e6, GHz: 1e9 };
const voltageFactor: Record<VoltageUnit, number> = { V: 1, mV: 1e-3 };
const toHz = (value: string, unit: FrequencyUnit) => {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed * hzFactor[unit] : 0;
};
const toVolt = (value: string, unit: VoltageUnit) => {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed * voltageFactor[unit] : 0;
};
const almostEqual = (left?: number | null, right?: number | null, tolerance = 1e-6) => {
  if (left == null || right == null) return false;
  return Math.abs(left - right) <= tolerance;
};
const stateBadge = (state: "on" | "off" | "ok" | "warning" | "blocked"): React.CSSProperties => {
  const palette = {
    on: ["#dbeafe", "var(--color-primary)"],
    off: ["var(--color-disabled-bg)", "var(--color-disabled-text)"],
    ok: ["var(--color-success-soft)", "var(--color-success)"],
    warning: ["#fff7ed", "#c2410c"],
    blocked: ["var(--color-danger-soft)", "var(--color-danger)"],
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

// ---------------------------------------------------------------------------
// Device catalog
// ---------------------------------------------------------------------------

interface DeviceInfo {
  id: string;
  name: string;
  kind: string;
  defaultAddress: string;
  expectedSn?: string;
}

interface Smb100aWorkbenchDraft {
  frequency: string;
  frequencyUnit: FrequencyUnit;
  powerDbm: string;
  rfOutputOn: boolean;
  modStateOn: boolean;
  lfOutputOn: boolean;
  lfVoltage: string;
  lfVoltageUnit: VoltageUnit;
  lfFrequency: string;
  lfFrequencyUnit: FrequencyUnit;
  lfShape: string;
  lfImpedance: string;
  fmEnabled: boolean;
  fmSource: string;
  fmMode: string;
  fmDeviation: string;
  fmDeviationUnit: FrequencyUnit;
  rfSweepStart: string;
  rfSweepStartUnit: FrequencyUnit;
  rfSweepStop: string;
  rfSweepStopUnit: FrequencyUnit;
  rfSweepStep: string;
  rfSweepStepUnit: FrequencyUnit;
  rfSweepDwellS: string;
  rfSweepOutputStartV: string;
  rfSweepOutputStopV: string;
}

interface Oe1022dChannelDraft {
  inputSource: string;
  inputShieldGrounding: string;
  inputCoupling: string;
  inputNotchFilter: string;
  dynamicReserve: string;
  sensitivity: string;
  timeConstantS: string;
  filterSlopeDbOct: string;
  referenceSource: string;
  externalRefTrigger: string;
  internalFrequencyHz: string;
  phaseDeg: string;
  sineoutMode: string;
  sineoutVoltageVrms: string;
  channelSource: string;
  offsetPercent: string;
  expand: string;
  speed: string;
}

interface Oe1022dWorkbenchDraft {
  chA: Oe1022dChannelDraft;
  chB: Oe1022dChannelDraft;
}

const DEVICE_CATALOG: DeviceInfo[] = [
  { id: "smb100a_main", name: "SMB100A", kind: "smb100a", defaultAddress: "169.254.2.20:5025" },
  { id: "oe1022d_main", name: "OE1022D", kind: "oe1022d", defaultAddress: "/dev/cu.usbmodem3361358734371" },
  { id: "cni_laser", name: "Laser", kind: "laser", defaultAddress: "/dev/cu.usbserial-LASER" },
  { id: "maynuo.mag_x", name: "Mag X", kind: "magnetic", defaultAddress: "auto", expectedSn: "2020" },
  { id: "maynuo.mag_y", name: "Mag Y", kind: "magnetic", defaultAddress: "auto", expectedSn: "2022" },
  { id: "maynuo.mag_z", name: "Mag Z", kind: "magnetic", defaultAddress: "auto", expectedSn: "2003" },
];

const defaultSmbDraft: Smb100aWorkbenchDraft = {
  frequency: "2.8565",
  frequencyUnit: "GHz",
  powerDbm: "-30",
  rfOutputOn: false,
  modStateOn: false,
  lfOutputOn: false,
  lfVoltage: "0",
  lfVoltageUnit: "V",
  lfFrequency: "500",
  lfFrequencyUnit: "Hz",
  lfShape: "方波 (SQUare)",
  lfImpedance: "低阻抗 (LOW)",
  fmEnabled: false,
  fmSource: "内部+外部 (INT,EXT)",
  fmMode: "正常 (NORMal)",
  fmDeviation: "3.5",
  fmDeviationUnit: "MHz",
  rfSweepStart: "2.8",
  rfSweepStartUnit: "GHz",
  rfSweepStop: "2.9",
  rfSweepStopUnit: "GHz",
  rfSweepStep: "1",
  rfSweepStepUnit: "MHz",
  rfSweepDwellS: "0.3",
  rfSweepOutputStartV: "0",
  rfSweepOutputStopV: "3",
};

const defaultOeChannelDraft: Oe1022dChannelDraft = {
  inputSource: "单端电压信号",
  inputShieldGrounding: "浮空",
  inputCoupling: "交流耦合",
  inputNotchFilter: "关闭所有陷波器",
  dynamicReserve: "正常 (NORMAL)",
  sensitivity: "100 mV/nA",
  timeConstantS: "0.3",
  filterSlopeDbOct: "12",
  referenceSource: "外部参考",
  externalRefTrigger: "过零检测",
  internalFrequencyHz: "102000",
  phaseDeg: "0",
  sineoutMode: "固定幅值模式",
  sineoutVoltageVrms: "1.000",
  channelSource: "A-R",
  offsetPercent: "0",
  expand: "1",
  speed: "慢速",
};

const defaultOeDraft: Oe1022dWorkbenchDraft = {
  chA: { ...defaultOeChannelDraft, referenceSource: "内部参考", internalFrequencyHz: "1" },
  chB: defaultOeChannelDraft,
};

// ---------------------------------------------------------------------------
// Main Page
// ---------------------------------------------------------------------------

export default function DevicesPage() {
  const [activeTab, setActiveTab] = useState<WorkbenchTab>("station");
  const [snapshot, setSnapshot] = useState<WorkbenchSnapshot | null>(null);
  const [connected, setConnected] = useState<Set<string>>(new Set());
  const [addresses, setAddresses] = useState<Record<string, string>>({});
  const [identities, setIdentities] = useState<Record<string, string>>({});
  const [refreshTick, setRefreshTick] = useState(0);
  const [zeroBaseline, setZeroBaseline] = useState<RuntimeZeroBaseline | null>(null);
  const [targetB, setTargetB] = useState({ x: "0", y: "0", z: "0" });
  const [magPackage, setMagPackage] = useState<MagneticXyzPackageStatus | null>(null);
  const [paraImportDraft, setParaImportDraft] = useState<unknown>(null);
  const [discoveryReport, setDiscoveryReport] = useState<DeviceDiscoveryReport | null>(null);
  const [autoBindReport, setAutoBindReport] = useState<AutoBindReport | null>(null);
  const [smbTcpTargets, setSmbTcpTargets] = useState("169.254.2.20:5025\n192.168.1.20:5025\n192.168.0.20:5025");
  const [discoveryBusy, setDiscoveryBusy] = useState<"discover" | "bind" | "connect" | null>(null);

  // Init addresses from defaults
  useEffect(() => {
    const init: Record<string, string> = {};
    for (const d of DEVICE_CATALOG) {
      init[d.id] = d.defaultAddress;
    }
    setAddresses(init);
  }, []);

  const refresh = useCallback(async () => {
    try {
      const snap: WorkbenchSnapshot = await invoke("get_workbench_state");
      setSnapshot(snap);
      setConnected(new Set(snap.locks_held));
      // Merge profile addresses
      setAddresses((prev) => {
        const merged = { ...prev };
        for (const [id, addr] of Object.entries(snap.profile_addresses)) {
          if (addr && addr !== "auto") merged[id] = addr;
        }
        return merged;
      });
      // Merge identities from report
      if (snap.report) {
        const idMap: Record<string, string> = {};
        for (const d of snap.report.devices) {
          if (d.identity_display || d.identity_raw) {
            idMap[d.device_id] = d.identity_display ?? d.identity_raw ?? "";
          }
        }
        setIdentities((prev) => ({ ...prev, ...idMap }));
      }
    } catch {
      setSnapshot(null);
      setConnected(new Set());
    }
  }, []);

  const refreshMagPackage = useCallback(async () => {
    const bxNt = Number.parseFloat(targetB.x);
    const byNt = Number.parseFloat(targetB.y);
    const bzNt = Number.parseFloat(targetB.z);
    try {
      const pkg: MagneticXyzPackageStatus = await invoke("magnetic_get_xyz_package_status", {
        bxNt: Number.isNaN(bxNt) ? 0 : bxNt,
        byNt: Number.isNaN(byNt) ? 0 : byNt,
        bzNt: Number.isNaN(bzNt) ? 0 : bzNt,
      });
      setMagPackage(pkg);
      if (pkg.runtime_zero_baseline) setZeroBaseline(pkg.runtime_zero_baseline);
    } catch {
      setMagPackage(null);
    }
  }, [targetB.x, targetB.y, targetB.z]);

  useEffect(() => {
    refresh();
    refreshMagPackage();
    const interval = setInterval(() => {
      refresh();
      refreshMagPackage();
    }, 3000);
    return () => clearInterval(interval);
  }, [refresh, refreshMagPackage]);

  const triggerRefresh = () => {
    refresh();
    refreshMagPackage();
    setRefreshTick((t) => t + 1);
  };

  const loadExample = async () => {
    try {
      await invoke("load_example_station_profile");
      triggerRefresh();
    } catch (e) {
      alert(String(e));
    }
  };

  const loadProfile = async () => {
    try {
      const path = (await invoke("pick_recipe_file")) as string | null;
      if (!path) return;
      await invoke("load_station_profile", { path });
      triggerRefresh();
    } catch (e) {
      alert(String(e));
    }
  };

  const checkAll = async () => {
    try {
      const report: StationPreflightReport = await invoke("run_station_preflight_cmd", {
        operatorApproved: true,
      });
      const ok = report.all_devices_reachable && report.all_identities_verified;
      alert(`Check All: ${ok ? "PASS" : "FAIL"} (${report.devices.length} devices)`);
      triggerRefresh();
    } catch (e) {
      alert(String(e));
    }
  };

  const releaseLocks = async () => {
    try {
      await invoke("release_all_locks");
      triggerRefresh();
    } catch (e) {
      alert(String(e));
    }
  };

  const discoverAllDevices = async () => {
    if (discoveryBusy) return;
    setDiscoveryBusy("discover");
    try {
      const request: DeviceProbeRequest = {
        requested_kinds: ["smb100a", "rf_source", "oe1022d", "magnetic", "laser"],
        smb100a_tcp_targets: [
          addresses["smb100a_main"],
          ...smbTcpTargets.split(/\s+/).map((target) => target.trim()).filter(Boolean),
        ].filter(Boolean),
        enable_usb_probe: false,
      };
      const report: DeviceDiscoveryReport = await invoke("discover_devices", {
        request,
      });
      setDiscoveryReport(report);
    } catch (e) {
      alert(String(e));
    } finally {
      setDiscoveryBusy(null);
    }
  };

  const autoBindDevices = async () => {
    if (discoveryBusy) return;
    setDiscoveryBusy("bind");
    try {
      const requestedRoles: DeviceRoleRequest[] = [
        { device_id: "smb100a_main", kind: "rf_source" },
        { device_id: "oe1022d_main", kind: "oe1022d" },
        { device_id: "maynuo.mag_x", kind: "magnetic", expected_sn: "2020" },
        { device_id: "maynuo.mag_y", kind: "magnetic", expected_sn: "2022" },
        { device_id: "maynuo.mag_z", kind: "magnetic", expected_sn: "2003" },
        { device_id: "cni_laser", kind: "laser" },
      ];
      const report: AutoBindReport = await invoke("auto_bind_discovered_devices", {
        requestedRoles,
        discovery: discoveryReport,
      });
      setAutoBindReport(report);
      const nextAddresses: Record<string, string> = {};
      for (const item of report.bound) {
        if (item.address) nextAddresses[item.device_id] = item.address;
      }
      setAddresses((prev) => ({ ...prev, ...nextAddresses }));
      const nextIdentities: Record<string, string> = {};
      for (const item of report.bound) {
        if (item.idn) nextIdentities[item.device_id] = item.idn;
      }
      setIdentities((prev) => ({ ...prev, ...nextIdentities }));
      triggerRefresh();
    } catch (e) {
      alert(String(e));
    } finally {
      setDiscoveryBusy(null);
    }
  };

  const connectBoundDevices = async () => {
    if (discoveryBusy) return;
    setDiscoveryBusy("connect");
    try {
      await invoke("connect_bound_devices");
      triggerRefresh();
    } catch (e) {
      alert(String(e));
    } finally {
      setDiscoveryBusy(null);
    }
  };

  const magAggregate = async <T,>(name: string, args?: Record<string, unknown>) => {
    try {
      const result = await invoke<T>(name, args);
      triggerRefresh();
      return result;
    } catch (e) {
      alert(String(e));
      return null;
    }
  };

  const initAllMag = async () => {
    await magAggregate<MagneticStatus[]>("magnetic_init_all");
  };

  const measureZeroAll = async () => {
    const baseline = await magAggregate<RuntimeZeroBaseline>("magnetic_measure_zero_all", {
      samplesPerAxis: 5,
    });
    if (baseline) setZeroBaseline(baseline);
  };

  const lockZeroAll = async () => {
    const baseline = await magAggregate<RuntimeZeroBaseline>("magnetic_lock_zero_all");
    if (baseline) setZeroBaseline(baseline);
  };

  const applyBVector = async () => {
    const bxNt = Number.parseFloat(targetB.x);
    const byNt = Number.parseFloat(targetB.y);
    const bzNt = Number.parseFloat(targetB.z);
    if ([bxNt, byNt, bzNt].some((v) => Number.isNaN(v))) {
      alert("Bx/By/Bz must be valid numbers.");
      return;
    }
    const result = await magAggregate<MagneticVectorApplyResult>("magnetic_apply_vector_field", {
      bxNt,
      byNt,
      bzNt,
    });
    if (result?.runtime_zero_baseline) setZeroBaseline(result.runtime_zero_baseline);
  };

  const cleanupAllMag = async () => {
    await magAggregate<MagneticStatus[]>("magnetic_cleanup_all");
    setZeroBaseline(null);
  };

  const importMagParaXml = async () => {
    try {
      const path = (await invoke("pick_recipe_file")) as string | null;
      if (!path) return;
      const draft = await invoke("import_magnetic_para_xml", { path });
      setParaImportDraft(draft);
      triggerRefresh();
    } catch (e) {
      alert(String(e));
    }
  };

  const profileLoaded = snapshot?.profile_loaded ?? false;
  const profileName = snapshot?.profile_name ?? null;

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>设备工作台</h1>

      {/* Profile header */}
      <div style={{ ...cardStyle, marginBottom: "var(--space-4)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-3)" }}>
          <div>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>Station Profile / 连接总览</div>
            <div style={{ ...smallMuted, marginTop: 2 }}>
              {profileLoaded
                ? `Profile: ${profileName ?? "unknown"} · 已加载`
                : "尚未加载 station.json，地址使用工作台默认值"}
            </div>
          </div>
          <span style={profileLoaded ? badgeOk : badgeNeutral}>
            {profileLoaded ? "已加载" : "未加载"}
          </span>
        </div>

        {profileLoaded && snapshot?.report && (
          <div style={{ ...smallMuted, marginBottom: "var(--space-3)" }}>
            Devices declared: {snapshot.report.devices.map((d) => d.device_id).join(", ")} ·
            Safety limits loaded: yes
          </div>
        )}

        <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
          <button onClick={loadProfile} style={btnSecondary}>加载 station.json…</button>
          <button onClick={loadExample} style={btnSecondary}>加载示例</button>
          <button onClick={checkAll} style={btnPrimary}>统一预检</button>
          <button onClick={releaseLocks} style={btnDanger}>释放设备锁</button>
          <button onClick={triggerRefresh} style={btnSecondary}>刷新全部</button>
        </div>
      </div>

      <WorkbenchTabs activeTab={activeTab} onChange={setActiveTab} />

      {activeTab === "station" && (
        <StationWorkbenchSummary
          snapshot={snapshot}
          connected={connected}
          addresses={addresses}
          identities={identities}
          discoveryReport={discoveryReport}
          autoBindReport={autoBindReport}
          smbTcpTargets={smbTcpTargets}
          busy={discoveryBusy}
          onSmbTcpTargetsChange={setSmbTcpTargets}
          onDiscover={discoverAllDevices}
          onAutoBind={autoBindDevices}
          onConnectBound={connectBoundDevices}
        />
      )}

      {activeTab === "smb100a" && (
        <Smb100aCard
          info={DEVICE_CATALOG[0]}
          isConnected={connected.has("smb100a_main")}
          address={addresses["smb100a_main"] ?? ""}
          identity={identities["smb100a_main"]}
          onAddressChange={(v) => setAddresses((p) => ({ ...p, smb100a_main: v }))}
          onConnectChange={triggerRefresh}
          refreshTick={refreshTick}
        />
      )}

      {activeTab === "oe1022d" && (
        <Oe1022dCard
          info={DEVICE_CATALOG[1]}
          isConnected={connected.has("oe1022d_main")}
          address={addresses["oe1022d_main"] ?? ""}
          identity={identities["oe1022d_main"]}
          onAddressChange={(v) => setAddresses((p) => ({ ...p, oe1022d_main: v }))}
          onConnectChange={triggerRefresh}
          refreshTick={refreshTick}
        />
      )}

      {activeTab === "laser" && (
        <LaserCard
          info={DEVICE_CATALOG[2]}
          isConnected={connected.has("cni_laser")}
          address={addresses["cni_laser"] ?? ""}
          identity={identities["cni_laser"]}
          onAddressChange={(v) => setAddresses((p) => ({ ...p, cni_laser: v }))}
          onConnectChange={triggerRefresh}
          refreshTick={refreshTick}
        />
      )}

      {/* Magnetic vector — full width section */}
      {activeTab === "magnetic" && (
      <div>
        <div style={{ ...sectionTitle, fontSize: "var(--font-size-lg)" }}>XYZ Magnetic Package / 三轴磁场参数包</div>
        <div style={{ ...cardStyle, marginBottom: "var(--space-4)" }}>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-4)", alignItems: "start" }}>
            <div>
              <div style={{ ...sectionTitle, marginBottom: "var(--space-3)" }}>Target field B (nT)</div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
                <label style={{ fontSize: "var(--font-size-xs)" }}>
                  Bx
                  <input value={targetB.x} onChange={(e) => setTargetB((p) => ({ ...p, x: e.target.value }))} style={{ ...inputStyle, width: "100%", marginTop: 4 }} />
                </label>
                <label style={{ fontSize: "var(--font-size-xs)" }}>
                  By
                  <input value={targetB.y} onChange={(e) => setTargetB((p) => ({ ...p, y: e.target.value }))} style={{ ...inputStyle, width: "100%", marginTop: 4 }} />
                </label>
                <label style={{ fontSize: "var(--font-size-xs)" }}>
                  Bz
                  <input value={targetB.z} onChange={(e) => setTargetB((p) => ({ ...p, z: e.target.value }))} style={{ ...inputStyle, width: "100%", marginTop: 4 }} />
                </label>
              </div>
              <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
                <button onClick={initAllMag} style={btnSecondary}>初始化三轴</button>
                <button onClick={measureZeroAll} style={btnSecondary}>测零三轴</button>
                <button onClick={lockZeroAll} style={btnSecondary}>锁零三轴</button>
                <button onClick={applyBVector} style={btnPrimary}>应用 B 向量</button>
                <button onClick={cleanupAllMag} style={btnDanger}>安全清理三轴</button>
                <button onClick={importMagParaXml} style={btnSecondary}>导入 para.xml</button>
              </div>
            </div>
            <div>
              <div style={{ ...sectionTitle, marginBottom: "var(--space-3)" }}>Runtime zero baseline / package state</div>
              {zeroBaseline ? (
                <div style={{ fontSize: "var(--font-size-xs)", display: "grid", gap: 6 }}>
                  <div><strong>Session:</strong> {zeroBaseline.session_id}</div>
                  <div><strong>Locked at:</strong> {zeroBaseline.locked_at}</div>
                  {(["x", "y", "z"] as const).map((axis) => {
                    const z = zeroBaseline.axes[axis];
                    return (
                      <div key={axis}>
                        <strong>{axis.toUpperCase()}:</strong>{" "}
                        {z ? `${(z.zero_mean_a * 1000).toFixed(3)} mA ± ${(z.zero_std_a * 1000).toFixed(3)} mA` : "—"}
                      </div>
                    );
                  })}
                </div>
              ) : (
                <div style={smallMuted}>No runtime zero baseline measured in this session.</div>
              )}
              <div style={{ ...smallMuted, marginTop: "var(--space-3)" }}>
                Calibration source: {magPackage?.calibration_source ?? "reverse_application/reverse_output/para.xml"}
              </div>
              <div style={{ marginTop: "var(--space-2)" }}>
                <span style={magPackage?.ready_to_apply ? badgeOk : badgeFail}>
                  {magPackage?.ready_to_apply ? "READY TO APPLY" : "BLOCKED / PREVIEW ONLY"}
                </span>
              </div>
              {magPackage && magPackage.blocked_reasons.length > 0 && (
                <div style={{ marginTop: "var(--space-2)", color: "var(--color-danger)", fontSize: "var(--font-size-xs)" }}>
                  {magPackage.blocked_reasons.slice(0, 4).join(" · ")}
                </div>
              )}
            </div>
          </div>
          {paraImportDraft ? (
            <pre style={{ ...smallMuted, marginTop: "var(--space-3)", whiteSpace: "pre-wrap" }}>
              {JSON.stringify(paraImportDraft, null, 2)}
            </pre>
          ) : null}
        </div>
        <MagneticPackageTable
          packageStatus={magPackage}
          deviceInfos={DEVICE_CATALOG.slice(3)}
          connected={connected}
          addresses={addresses}
          identities={identities}
          onAddressChange={(id, value) => setAddresses((p) => ({ ...p, [id]: value }))}
          onChanged={triggerRefresh}
        />
      </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Workbench tab helpers
// ---------------------------------------------------------------------------

function WorkbenchTabs({ activeTab, onChange }: { activeTab: WorkbenchTab; onChange: (tab: WorkbenchTab) => void }) {
  const tabs: Array<[WorkbenchTab, string]> = [
    ["station", "Station"],
    ["magnetic", "磁场"],
    ["smb100a", "SMB100A"],
    ["oe1022d", "OE1022D"],
    ["laser", "激光"],
  ];
  return (
    <div style={{ ...cardStyle, display: "flex", gap: "var(--space-2)", flexWrap: "wrap", padding: "var(--space-3)", marginBottom: "var(--space-4)" }}>
      {tabs.map(([tab, label]) => (
        <button key={tab} onClick={() => onChange(tab)} style={activeTab === tab ? btnPrimary : btnSecondary}>{label}</button>
      ))}
    </div>
  );
}

function StationWorkbenchSummary({
  snapshot,
  connected,
  addresses,
  identities,
  discoveryReport,
  autoBindReport,
  smbTcpTargets,
  busy,
  onSmbTcpTargetsChange,
  onDiscover,
  onAutoBind,
  onConnectBound,
}: {
  snapshot: WorkbenchSnapshot | null;
  connected: Set<string>;
  addresses: Record<string, string>;
  identities: Record<string, string>;
  discoveryReport: DeviceDiscoveryReport | null;
  autoBindReport: AutoBindReport | null;
  smbTcpTargets: string;
  busy: "discover" | "bind" | "connect" | null;
  onSmbTcpTargetsChange: (value: string) => void;
  onDiscover: () => void;
  onAutoBind: () => void;
  onConnectBound: () => void;
}) {
  return (
    <div style={{ display: "grid", gap: "var(--space-4)" }}>
      <div style={cardStyle}>
        <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>Station 状态</h2>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(190px, 1fr))", gap: "var(--space-3)", fontSize: "var(--font-size-xs)" }}>
          <div><strong>Profile:</strong> {snapshot?.profile_loaded ? snapshot.profile_name ?? "loaded" : "未加载"}</div>
          <div><strong>Preflight:</strong> {snapshot?.preflight_passed ? <span style={stateBadge("ok")}>PASS</span> : <span style={stateBadge("off")}>未通过</span>}</div>
          <div><strong>Locks:</strong> {snapshot?.locks_held.length ?? 0}</div>
          <div><strong>连接设备:</strong> {connected.size}</div>
        </div>
      </div>
      <div style={cardStyle}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "start", gap: "var(--space-3)", marginBottom: "var(--space-3)" }}>
          <div>
            <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-1)" }}>设备发现与自动绑定</h2>
            <div style={smallMuted}>先扫描串口与 SMB100A TCP 候选，再按 probe profile 自动绑定角色；只发送只读 IDN，不开 RF、不改设备输出。</div>
          </div>
          <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap", justifyContent: "end" }}>
            <button onClick={onDiscover} disabled={busy !== null} style={busy ? btnDisabled : btnSecondary}>
              {busy === "discover" ? "扫描中…" : "扫描所有设备"}
            </button>
            <button onClick={onAutoBind} disabled={busy !== null} style={busy ? btnDisabled : btnPrimary}>
              {busy === "bind" ? "绑定中…" : "自动绑定识别设备"}
            </button>
            <button onClick={onConnectBound} disabled={busy !== null} style={busy ? btnDisabled : btnSecondary}>
              {busy === "connect" ? "连接中…" : "连接已绑定设备"}
            </button>
          </div>
        </div>
        {busy && (
          <div style={{ color: "var(--color-primary)", fontSize: "var(--font-size-xs)", marginBottom: "var(--space-2)" }}>
            正在执行 {busy === "discover" ? "设备发现" : busy === "bind" ? "自动绑定" : "连接已绑定设备"}，请等待当前操作完成。
          </div>
        )}
        <label style={{ display: "grid", gap: 6, marginBottom: "var(--space-3)", fontSize: "var(--font-size-xs)" }}>
          <strong>SMB100A TCP 候选地址（每行一个 host:port）</strong>
          <textarea
            value={smbTcpTargets}
            onChange={(e) => onSmbTcpTargetsChange(e.target.value)}
            rows={3}
            style={{ ...inputStyle, width: "100%", resize: "vertical", fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace' }}
          />
          <span style={smallMuted}>当前 SMB100A 地址输入框也会作为候选；未识别时仍可手动填写 TCP 地址后连接。</span>
        </label>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: "var(--space-3)", marginBottom: "var(--space-3)", fontSize: "var(--font-size-xs)" }}>
          <div><strong>串口端口:</strong> {discoveryReport?.serial_ports.length ?? 0}</div>
          <div><strong>TCP 候选:</strong> {discoveryReport?.tcp_targets.length ?? 0}</div>
          <div><strong>识别设备:</strong> {discoveryReport?.devices.length ?? 0}</div>
          <div><strong>已绑定:</strong> {autoBindReport?.bound.length ?? 0}</div>
          <div><strong>阻塞:</strong> {autoBindReport?.blocked.length ?? 0}</div>
        </div>
        {autoBindReport && autoBindReport.blocked.length > 0 && (
          <div style={{ color: "var(--color-danger)", fontSize: "var(--font-size-xs)", marginBottom: "var(--space-2)" }}>
            {autoBindReport.blocked.join(" · ")}
          </div>
        )}
        {discoveryReport && !discoveryReport.devices.some((device) => device.suggested_role === "smb100a_main") && (
          <div style={{ color: "#c2410c", fontSize: "var(--font-size-xs)", marginBottom: "var(--space-2)" }}>
            未识别到 SMB100A；可继续使用手动 TCP 地址连接。
          </div>
        )}
        {discoveryReport && discoveryReport.warnings.length > 0 && (
          <div style={{ ...smallMuted, marginBottom: "var(--space-2)" }}>
            Warnings: {discoveryReport.warnings.slice(0, 6).join(" · ")}
          </div>
        )}
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-xs)", minWidth: 1100 }}>
            <thead>
              <tr style={{ color: "var(--color-text-muted)", textAlign: "left", borderBottom: "1px solid var(--color-border)" }}>
                <th style={{ padding: 8 }}>transport</th>
                <th style={{ padding: 8 }}>address</th>
                <th style={{ padding: 8 }}>detected_kind</th>
                <th style={{ padding: 8 }}>model</th>
                <th style={{ padding: 8 }}>idn</th>
                <th style={{ padding: 8 }}>serial_number</th>
                <th style={{ padding: 8 }}>confidence</th>
                <th style={{ padding: 8 }}>suggested_role</th>
                <th style={{ padding: 8 }}>status</th>
              </tr>
            </thead>
            <tbody>
              {(discoveryReport?.devices ?? []).map((device) => (
                <tr key={`${device.transport}.${device.address}.${device.detected_kind}.${device.suggested_role ?? ""}`}>
                  <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>{device.transport}</td>
                  <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>{device.address}</td>
                  <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>{device.detected_kind}</td>
                  <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>{device.model ?? "—"}</td>
                  <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)", maxWidth: 360, overflowWrap: "anywhere" }}>{device.idn ?? "—"}</td>
                  <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>{device.serial_number ?? "—"}</td>
                  <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>
                    <span style={stateBadge(device.confidence === "high" ? "ok" : device.confidence === "medium" ? "warning" : "off")}>{device.confidence}</span>
                  </td>
                  <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>{device.suggested_role ?? "—"}</td>
                  <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>{device.status}</td>
                </tr>
              ))}
              {!discoveryReport && (
                <tr>
                  <td colSpan={9} style={{ padding: 8, color: "var(--color-text-muted)" }}>尚未扫描。</td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
      <div style={{ ...cardStyle, overflowX: "auto" }}>
        <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>设备地址与身份</h2>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-xs)", minWidth: 820 }}>
          <thead>
            <tr style={{ color: "var(--color-text-muted)", textAlign: "left", borderBottom: "1px solid var(--color-border)" }}>
              <th style={{ padding: 8 }}>设备</th>
              <th style={{ padding: 8 }}>状态</th>
              <th style={{ padding: 8 }}>地址</th>
              <th style={{ padding: 8 }}>身份 / IDN</th>
            </tr>
          </thead>
          <tbody>
            {DEVICE_CATALOG.map((device) => (
              <tr key={device.id}>
                <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>{device.name}<div style={smallMuted}>{device.id}</div></td>
                <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>
                  <span style={connected.has(device.id) ? stateBadge("ok") : stateBadge("off")}>{connected.has(device.id) ? "已连接/持锁" : "未连接"}</span>
                </td>
                <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)" }}>{addresses[device.id] ?? device.defaultAddress}</td>
                <td style={{ padding: 8, borderBottom: "1px solid var(--color-border)", maxWidth: 360, overflowWrap: "anywhere" }}>{identities[device.id] ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared card helpers
// ---------------------------------------------------------------------------

function useConnect(
  info: DeviceInfo,
  _isConnected: boolean,
  address: string,
  onConnectChange: () => void,
) {
  const [connecting, setConnecting] = useState(false);

  const doConnect = async () => {
    setConnecting(true);
    try {
      const idn: string = await invoke("connect_single_device", {
        deviceId: info.id,
        address,
        kind: info.kind,
      });
      alert(`Connected: ${idn}`);
      onConnectChange();
    } catch (e) {
      alert(String(e));
    } finally {
      setConnecting(false);
    }
  };

  const doDisconnect = async () => {
    try {
      await invoke("disconnect_single_device", { deviceId: info.id });
      onConnectChange();
    } catch (e) {
      alert(String(e));
    }
  };

  const doIdentify = async () => {
    // Re-probe identity via connect (lightweight)
    try {
      const idn: string = await invoke("connect_single_device", {
        deviceId: info.id,
        address,
        kind: info.kind,
      });
      alert(`Identity: ${idn}`);
      onConnectChange();
    } catch (e) {
      alert(String(e));
    }
  };

  return { connecting, doConnect, doDisconnect, doIdentify };
}

function DraftField({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <label style={{ display: "grid", gap: 5, fontSize: "var(--font-size-xs)" }}>
      {label}
      <input value={value} onChange={(event) => onChange(event.target.value)} style={inputStyle} />
    </label>
  );
}

function UnitField({
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

function SelectDraft({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: string[];
  onChange: (value: string) => void;
}) {
  return (
    <label style={{ display: "grid", gap: 5, fontSize: "var(--font-size-xs)" }}>
      {label}
      <select value={value} onChange={(event) => onChange(event.target.value)} style={inputStyle}>
        {options.map((option) => <option key={option} value={option}>{option}</option>)}
      </select>
    </label>
  );
}

function ToggleDraft({
  label,
  checked,
  onChange,
  note,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  note?: string;
}) {
  return (
    <label style={{ display: "grid", gap: 5, fontSize: "var(--font-size-xs)" }}>
      <span>{label}</span>
      <span style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
        <span style={stateBadge(checked ? "on" : "off")}>{checked ? "ON" : "OFF"}</span>
      </span>
      {note && <span style={smallMuted}>{note}</span>}
    </label>
  );
}

// ---------------------------------------------------------------------------
// SMB100A Card
// ---------------------------------------------------------------------------

function Smb100aCard({
  info,
  isConnected,
  address,
  identity,
  onAddressChange,
  onConnectChange,
  refreshTick,
}: {
  info: DeviceInfo;
  isConnected: boolean;
  address: string;
  identity?: string;
  onAddressChange: (v: string) => void;
  onConnectChange: () => void;
  refreshTick: number;
}) {
  const [status, setStatus] = useState<Smb100aStatus | null>(null);
  const [freq, setFreq] = useState("");
  const [pwr, setPwr] = useState("");
  const [draft, setDraft] = useState<Smb100aWorkbenchDraft>(defaultSmbDraft);
  const [draftReady, setDraftReady] = useState(false);
  const [applyMessage, setApplyMessage] = useState<string | null>(null);

  const { connecting, doConnect, doDisconnect, doIdentify } = useConnect(info, isConnected, address, onConnectChange);

  const fetchStatus = useCallback(async () => {
    try {
      const s: Smb100aStatus = await invoke("smb100a_get_status", { deviceId: info.id });
      setStatus(s);
    } catch {
      setStatus(null);
    }
  }, [info.id]);

  useEffect(() => {
    if (isConnected) fetchStatus();
  }, [fetchStatus, refreshTick, isConnected]);

  useEffect(() => {
    let cancelled = false;
    const loadDraft = async () => {
      try {
        const drafts = await invoke<Record<string, unknown>>("get_device_preset_drafts");
        const saved = drafts.smb100a;
        if (!cancelled && saved && typeof saved === "object") {
          setDraft({ ...defaultSmbDraft, ...(saved as Partial<Smb100aWorkbenchDraft>) });
        }
      } finally {
        if (!cancelled) setDraftReady(true);
      }
    };
    void loadDraft();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!draftReady) return;
    void invoke("set_device_preset_draft", { device: "smb100a", draft }).catch(() => undefined);
  }, [draft, draftReady]);

  const cmd = async (name: string, args?: Record<string, unknown>) => {
    try {
      const s: Smb100aStatus = await invoke(name, { deviceId: info.id, ...args });
      setStatus(s);
    } catch (e) {
      alert(String(e));
    }
  };

  const updateDraft = (patch: Partial<Smb100aWorkbenchDraft>) => setDraft((current) => ({ ...current, ...patch }));
  const draftFrequencyHz = toHz(draft.frequency, draft.frequencyUnit);
  const draftLfFrequencyHz = toHz(draft.lfFrequency, draft.lfFrequencyUnit);
  const draftLfVoltageV = toVolt(draft.lfVoltage, draft.lfVoltageUnit);
  const draftFmDeviationHz = toHz(draft.fmDeviation, draft.fmDeviationUnit);
  const applyWorkbenchDraft = async () => {
    if (!isConnected) return;
    setApplyMessage(null);
    try {
      let next: Smb100aStatus = await invoke("smb100a_set_output", { deviceId: info.id, on: false });
      next = await invoke("smb100a_set_frequency", { deviceId: info.id, hz: draftFrequencyHz });
      const power = Number.parseFloat(draft.powerDbm);
      if (Number.isFinite(power)) {
        next = await invoke("smb100a_set_power", { deviceId: info.id, dbm: power });
      }
      next = await invoke("smb100a_set_lf", {
        deviceId: info.id,
        frequencyHz: draftLfFrequencyHz,
        voltageV: draftLfVoltageV,
        outputOn: draft.lfOutputOn,
      });
      next = await invoke("smb100a_set_fm", {
        deviceId: info.id,
        enabled: draft.fmEnabled,
        deviationHz: draftFmDeviationHz,
      });
      setStatus(next);
      setApplyMessage("已应用主设置/LF/FM；RF 输出保持 OFF，RF sweep 参数已保存为工作台草稿。");
    } catch (e) {
      setApplyMessage(`阻塞：${String(e)}`);
    }
  };

  return (
    <div style={cardStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-2)" }}>
        <h2 style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>SMB100A</h2>
        <span style={isConnected ? badgeOk : badgeNeutral}>{isConnected ? "CONNECTED" : "DISCONNECTED"}</span>
      </div>

      {/* Address */}
      <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center", marginBottom: "var(--space-3)" }}>
        <span style={{ ...smallMuted, whiteSpace: "nowrap" }}>Address:</span>
        <input type="text" value={address} onChange={(e) => onAddressChange(e.target.value)} style={inputStyle} disabled={isConnected} />
        {!isConnected ? (
          <button onClick={doConnect} disabled={connecting} style={btnPrimary}>{connecting ? "…" : "Connect"}</button>
        ) : (
          <button onClick={doDisconnect} style={btnDanger}>Disconnect</button>
        )}
        <button onClick={doIdentify} disabled={!isConnected} style={isConnected ? btnSecondary : btnDisabled}>Identify</button>
        <button onClick={fetchStatus} disabled={!isConnected} style={isConnected ? btnSecondary : btnDisabled}>Refresh</button>
      </div>

      {/* Identity */}
      <div style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-xs)", lineHeight: 1.6 }}>
        <div><strong>IDN:</strong> {identity ?? status?.last_readback_time ? (status ? "verified" : "—") : "—"}</div>
        <div><strong>Connection:</strong> {isConnected ? "available" : "unavailable"}</div>
        <div><strong>Last readback:</strong> {status?.last_readback_time ?? "—"}</div>
      </div>

      {/* Readback */}
      {status && (
        <div style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-xs)", display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4 }}>
          <div><strong>OUTP:</strong> <span style={stateBadge(status.output_on ? "on" : "off")}>{status.output_on ? "ON" : "OFF"}</span></div>
          <div><strong>FREQ:</strong> {status.frequency_hz ? `${(status.frequency_hz / 1e6).toFixed(3)} MHz` : "—"}</div>
          <div><strong>POW:</strong> {status.power_dbm !== undefined ? `${status.power_dbm} dBm` : "—"}</div>
          <div><strong>MOD:STAT:</strong> <span style={stateBadge(status.modulation_on ? "on" : "off")}>{status.modulation_on ? "ON" : "OFF"}</span></div>
          <div><strong>FM:</strong> <span style={stateBadge(status.fm_enabled ? "on" : "off")}>{status.fm_enabled ? "ON" : "OFF"}</span></div>
          <div><strong>SYST:ERR:</strong> {status.error_queue.length > 0 ? status.error_queue.join(", ") : "none"}</div>
        </div>
      )}

      <div style={{ borderTop: "1px solid var(--color-border)", paddingTop: "var(--space-3)", display: "grid", gap: "var(--space-3)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "var(--space-2)" }}>
          <div>
            <div style={sectionTitle}>SMB100A 实时配置草稿</div>
            <div style={smallMuted}>编辑不会立即生效；点击“应用到设备”后通过 Tauri typed commands 下发并读回。</div>
          </div>
          <button onClick={applyWorkbenchDraft} disabled={!isConnected} style={isConnected ? btnPrimary : btnDisabled}>应用到设备</button>
        </div>
        {applyMessage && <div style={{ color: applyMessage.startsWith("阻塞") ? "var(--color-danger)" : "var(--color-success)", fontSize: "var(--font-size-xs)" }}>{applyMessage}</div>}
        <details open>
          <summary style={{ cursor: "pointer", fontWeight: 600 }}>主设置</summary>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: "var(--space-2)", marginTop: "var(--space-2)" }}>
            <UnitField label="频率" value={draft.frequency} unit={draft.frequencyUnit} units={frequencyUnits} onValue={(value) => updateDraft({ frequency: value })} onUnit={(unit) => updateDraft({ frequencyUnit: unit as FrequencyUnit })} />
            <DraftField label="电平 dBm" value={draft.powerDbm} onChange={(value) => updateDraft({ powerDbm: value })} />
            <ToggleDraft label="RF 输出开关草稿" checked={draft.rfOutputOn} onChange={(checked) => updateDraft({ rfOutputOn: checked })} note="应用草稿不会自动 RF ON" />
            <ToggleDraft label="调制总开关草稿" checked={draft.modStateOn} onChange={(checked) => updateDraft({ modStateOn: checked })} note="当前 typed command 尚未单独下发 MOD:STAT" />
          </div>
          {status && (
            <div style={{ ...smallMuted, marginTop: "var(--space-2)" }}>
              Readback 对比：频率 {almostEqual(status.frequency_hz, draftFrequencyHz, 1) ? "已生效" : "不一致"}；功率 {almostEqual(status.power_dbm, Number.parseFloat(draft.powerDbm), 0.02) ? "已生效" : "不一致/未读回"}
            </div>
          )}
        </details>
        <details open>
          <summary style={{ cursor: "pointer", fontWeight: 600 }}>LF / FM</summary>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: "var(--space-2)", marginTop: "var(--space-2)" }}>
            <ToggleDraft label="LF 输出" checked={draft.lfOutputOn} onChange={(checked) => updateDraft({ lfOutputOn: checked })} />
            <UnitField label="LF 电压" value={draft.lfVoltage} unit={draft.lfVoltageUnit} units={voltageUnits} onValue={(value) => updateDraft({ lfVoltage: value })} onUnit={(unit) => updateDraft({ lfVoltageUnit: unit as VoltageUnit })} />
            <UnitField label="LF 频率" value={draft.lfFrequency} unit={draft.lfFrequencyUnit} units={frequencyUnits} onValue={(value) => updateDraft({ lfFrequency: value })} onUnit={(unit) => updateDraft({ lfFrequencyUnit: unit as FrequencyUnit })} />
            <SelectDraft label="LF 波形" value={draft.lfShape} options={["正弦 (SINE)", "方波 (SQUare)", "锯齿 (SAWtooth)"]} onChange={(value) => updateDraft({ lfShape: value })} />
            <SelectDraft label="源阻抗" value={draft.lfImpedance} options={["低阻抗 (LOW)", "高阻抗 (HIGH)"]} onChange={(value) => updateDraft({ lfImpedance: value })} />
            <ToggleDraft label="FM 调制" checked={draft.fmEnabled} onChange={(checked) => updateDraft({ fmEnabled: checked })} />
            <SelectDraft label="FM 调制源" value={draft.fmSource} options={["内部 (INTernal)", "外部 (EXTernal)", "内部+外部 (INT,EXT)"]} onChange={(value) => updateDraft({ fmSource: value })} />
            <SelectDraft label="FM 调制模式" value={draft.fmMode} options={["正常 (NORMal)", "高偏差 (HDEViation)", "低噪声 (LNOise)"]} onChange={(value) => updateDraft({ fmMode: value })} />
            <UnitField label="FM 偏差" value={draft.fmDeviation} unit={draft.fmDeviationUnit} units={frequencyUnits} onValue={(value) => updateDraft({ fmDeviation: value })} onUnit={(unit) => updateDraft({ fmDeviationUnit: unit as FrequencyUnit })} />
          </div>
        </details>
        <details>
          <summary style={{ cursor: "pointer", fontWeight: 600 }}>RF frequency sweep 草稿</summary>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: "var(--space-2)", marginTop: "var(--space-2)" }}>
            <UnitField label="Start" value={draft.rfSweepStart} unit={draft.rfSweepStartUnit} units={frequencyUnits} onValue={(value) => updateDraft({ rfSweepStart: value })} onUnit={(unit) => updateDraft({ rfSweepStartUnit: unit as FrequencyUnit })} />
            <UnitField label="Stop" value={draft.rfSweepStop} unit={draft.rfSweepStopUnit} units={frequencyUnits} onValue={(value) => updateDraft({ rfSweepStop: value })} onUnit={(unit) => updateDraft({ rfSweepStopUnit: unit as FrequencyUnit })} />
            <UnitField label="Step" value={draft.rfSweepStep} unit={draft.rfSweepStepUnit} units={frequencyUnits} onValue={(value) => updateDraft({ rfSweepStep: value })} onUnit={(unit) => updateDraft({ rfSweepStepUnit: unit as FrequencyUnit })} />
            <DraftField label="Dwell s" value={draft.rfSweepDwellS} onChange={(value) => updateDraft({ rfSweepDwellS: value })} />
            <DraftField label="Sweep output start V" value={draft.rfSweepOutputStartV} onChange={(value) => updateDraft({ rfSweepOutputStartV: value })} />
            <DraftField label="Sweep output stop V" value={draft.rfSweepOutputStopV} onChange={(value) => updateDraft({ rfSweepOutputStopV: value })} />
          </div>
        </details>
      </div>

      {/* Controls */}
      {isConnected && (
        <div style={{ borderTop: "1px solid var(--color-border)", paddingTop: "var(--space-3)", display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
          <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
            <button onClick={() => cmd("smb100a_set_output", { on: false })} style={btnDanger}>RF OFF</button>
            <button onClick={() => cmd("smb100a_set_output", { on: true })} style={btnPrimary}>RF ON</button>
            <button onClick={() => cmd("smb100a_apply_safe_config")} style={btnSecondary}>Apply Safe Config</button>
          </div>
          <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
            <input type="text" placeholder="Freq (Hz)" value={freq} onChange={(e) => setFreq(e.target.value)} style={{ ...inputStyle, width: 100 }} />
            <button onClick={() => { const hz = parseFloat(freq); if (!Number.isNaN(hz)) cmd("smb100a_set_frequency", { hz }); }} style={btnSecondary}>Set Freq</button>
            <input type="text" placeholder="Power (dBm)" value={pwr} onChange={(e) => setPwr(e.target.value)} style={{ ...inputStyle, width: 100 }} />
            <button onClick={() => { const dbm = parseFloat(pwr); if (!Number.isNaN(dbm)) cmd("smb100a_set_power", { dbm }); }} style={btnSecondary}>Set Power</button>
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// OE1022D Card
// ---------------------------------------------------------------------------

function Oe1022dCard({
  info,
  isConnected,
  address,
  identity,
  onAddressChange,
  onConnectChange,
  refreshTick,
}: {
  info: DeviceInfo;
  isConnected: boolean;
  address: string;
  identity?: string;
  onAddressChange: (v: string) => void;
  onConnectChange: () => void;
  refreshTick: number;
}) {
  const [status, setStatus] = useState<Oe1022dStatus | null>(null);
  const [draft, setDraft] = useState<Oe1022dWorkbenchDraft>(defaultOeDraft);
  const [draftReady, setDraftReady] = useState(false);
  const [applyMessage, setApplyMessage] = useState<string | null>(null);

  const { connecting, doConnect, doDisconnect, doIdentify } = useConnect(info, isConnected, address, onConnectChange);

  const fetchStatus = useCallback(async () => {
    try {
      const s: Oe1022dStatus = await invoke("oe1022d_get_status", { deviceId: info.id });
      setStatus(s);
    } catch {
      setStatus(null);
    }
  }, [info.id]);

  useEffect(() => {
    if (isConnected) fetchStatus();
  }, [fetchStatus, refreshTick, isConnected]);

  useEffect(() => {
    let cancelled = false;
    const loadDraft = async () => {
      try {
        const drafts = await invoke<Record<string, unknown>>("get_device_preset_drafts");
        const saved = drafts.oe1022d;
        if (!cancelled && saved && typeof saved === "object") {
          setDraft({ ...defaultOeDraft, ...(saved as Partial<Oe1022dWorkbenchDraft>) });
        }
      } finally {
        if (!cancelled) setDraftReady(true);
      }
    };
    void loadDraft();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!draftReady) return;
    void invoke("set_device_preset_draft", { device: "oe1022d", draft }).catch(() => undefined);
  }, [draft, draftReady]);

  const cmd = async (name: string, args?: Record<string, unknown>) => {
    try {
      const s: Oe1022dStatus = await invoke(name, { deviceId: info.id, ...args });
      setStatus(s);
    } catch (e) {
      alert(String(e));
    }
  };

  const updateChannel = (channel: keyof Oe1022dWorkbenchDraft, patch: Partial<Oe1022dChannelDraft>) => {
    setDraft((current) => ({ ...current, [channel]: { ...current[channel], ...patch } }));
  };

  const applyOeDraft = async () => {
    if (!isConnected) return;
    setApplyMessage(null);
    const timeConstantS = Number.parseFloat(draft.chB.timeConstantS);
    const slopeDbOct = Number.parseInt(draft.chB.filterSlopeDbOct, 10);
    const phaseDeg = Number.parseFloat(draft.chB.phaseDeg);
    try {
      let next: Oe1022dStatus = await invoke("oe1022d_set_filter", {
        deviceId: info.id,
        timeConstantS: Number.isFinite(timeConstantS) ? timeConstantS : 0.3,
        slopeDbOct: Number.isFinite(slopeDbOct) ? slopeDbOct : 12,
      });
      next = await invoke("oe1022d_set_reference", {
        deviceId: info.id,
        source: draft.chB.referenceSource === "内部参考" ? "Internal" : "External",
        phaseDeg: Number.isFinite(phaseDeg) ? phaseDeg : 0,
      });
      setStatus(next);
      setApplyMessage("已应用 Ch-B 滤波/参考配置；Ch-A 和输出/公式配置已保存为草稿，等待后续 typed command 扩展。");
    } catch (e) {
      setApplyMessage(`阻塞：${String(e)}`);
    }
  };

  return (
    <div style={cardStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-2)" }}>
        <h2 style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>OE1022D</h2>
        <span style={isConnected ? badgeOk : badgeNeutral}>{isConnected ? "CONNECTED" : "DISCONNECTED"}</span>
      </div>

      <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center", marginBottom: "var(--space-3)" }}>
        <span style={{ ...smallMuted, whiteSpace: "nowrap" }}>Port:</span>
        <input type="text" value={address} onChange={(e) => onAddressChange(e.target.value)} style={inputStyle} disabled={isConnected} />
        {!isConnected ? (
          <button onClick={doConnect} disabled={connecting} style={btnPrimary}>{connecting ? "…" : "Connect"}</button>
        ) : (
          <button onClick={doDisconnect} style={btnDanger}>Disconnect</button>
        )}
        <button onClick={doIdentify} disabled={!isConnected} style={isConnected ? btnSecondary : btnDisabled}>Identify</button>
        <button onClick={fetchStatus} disabled={!isConnected} style={isConnected ? btnSecondary : btnDisabled}>Refresh</button>
      </div>

      <div style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-xs)", lineHeight: 1.6 }}>
        <div><strong>IDN:</strong> {identity ?? "—"}</div>
        <div><strong>Last readback:</strong> {status?.last_readback_time ?? "—"}</div>
      </div>

      {status && (
        <div style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-xs)", display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4 }}>
          <div><strong>Ref:</strong> {status.reference_source ?? "—"}</div>
          <div><strong>TC:</strong> {status.time_constant_s ? `${status.time_constant_s} s` : "—"}</div>
          <div><strong>Slope:</strong> {status.filter_slope_db_oct ? `${status.filter_slope_db_oct} dB/oct` : "—"}</div>
          <div><strong>Coupling:</strong> {status.input_coupling ?? "—"}</div>
          <div><strong>Notch:</strong> {status.input_notch ?? "—"}</div>
          <div><strong>PLL:</strong> <span style={stateBadge(status.pll_locked ? "ok" : "warning")}>{status.pll_locked ? "LOCKED" : "UNLOCKED"}</span></div>
          <div><strong>Overload:</strong> <span style={stateBadge(status.input_overload ? "blocked" : "ok")}>{status.input_overload ? "YES" : "NO"}</span></div>
          <div><strong>Gain OV:</strong> <span style={stateBadge(status.gain_overload ? "blocked" : "ok")}>{status.gain_overload ? "YES" : "NO"}</span></div>
        </div>
      )}

      <div style={{ borderTop: "1px solid var(--color-border)", paddingTop: "var(--space-3)", display: "grid", gap: "var(--space-3)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "var(--space-2)" }}>
          <div>
            <div style={sectionTitle}>OE1022D 双通道配置草稿</div>
            <div style={smallMuted}>Ch-A/Ch-B 分开保存；当前真实下发 typed command 仍以 Ch-B 为主。</div>
          </div>
          <button onClick={applyOeDraft} disabled={!isConnected} style={isConnected ? btnPrimary : btnDisabled}>应用 Ch-B 到设备</button>
        </div>
        {applyMessage && <div style={{ color: applyMessage.startsWith("阻塞") ? "var(--color-danger)" : "var(--color-success)", fontSize: "var(--font-size-xs)" }}>{applyMessage}</div>}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(360px, 1fr))", gap: "var(--space-3)" }}>
          <OeChannelWorkbenchEditor title="Ch-A" channel={draft.chA} onChange={(patch) => updateChannel("chA", patch)} applyEnabled={false} />
          <OeChannelWorkbenchEditor title="Ch-B" channel={draft.chB} onChange={(patch) => updateChannel("chB", patch)} applyEnabled />
        </div>
        {status && (
          <div style={smallMuted}>
            Ch-B readback 对比：TC {almostEqual(status.time_constant_s, Number.parseFloat(draft.chB.timeConstantS), 0.02) ? "已生效" : "不一致/未读回"}；
            slope {status.filter_slope_db_oct === Number.parseInt(draft.chB.filterSlopeDbOct, 10) ? "已生效" : "不一致/未读回"}
          </div>
        )}
      </div>

      {isConnected && (
        <div style={{ borderTop: "1px solid var(--color-border)", paddingTop: "var(--space-3)", display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
          <button onClick={() => cmd("oe1022d_apply_default_config")} style={btnSecondary}>Apply Default Config</button>
          <button onClick={() => cmd("oe1022d_auto_phase")} style={btnSecondary}>Auto Phase</button>
          <button onClick={fetchStatus} style={btnSecondary}>Refresh</button>
        </div>
      )}
    </div>
  );
}

function OeChannelWorkbenchEditor({
  title,
  channel,
  onChange,
  applyEnabled,
}: {
  title: string;
  channel: Oe1022dChannelDraft;
  onChange: (patch: Partial<Oe1022dChannelDraft>) => void;
  applyEnabled: boolean;
}) {
  const isExternal = channel.referenceSource === "外部参考";
  const isFixedSine = channel.sineoutMode === "固定幅值模式";
  return (
    <div style={{ border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", padding: "var(--space-3)", display: "grid", gap: "var(--space-3)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h3 style={{ fontSize: "var(--font-size-md)" }}>{title}</h3>
        <span style={applyEnabled ? stateBadge("ok") : stateBadge("warning")}>{applyEnabled ? "可应用到设备" : "草稿保存"}</span>
      </div>
      <details open>
        <summary style={{ cursor: "pointer", fontWeight: 600 }}>输入 / 滤波</summary>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: "var(--space-2)", marginTop: "var(--space-2)" }}>
          <SelectDraft label="输入信号源" value={channel.inputSource} options={["单端电压信号", "差分电压信号", "电流信号"]} onChange={(value) => onChange({ inputSource: value })} />
          <SelectDraft label="输入屏蔽接地" value={channel.inputShieldGrounding} options={["浮空", "接地"]} onChange={(value) => onChange({ inputShieldGrounding: value })} />
          <SelectDraft label="输入耦合" value={channel.inputCoupling} options={["交流耦合", "直流耦合"]} onChange={(value) => onChange({ inputCoupling: value })} />
          <SelectDraft label="输入陷波器" value={channel.inputNotchFilter} options={["关闭所有陷波器", "50 Hz", "100 Hz", "50/100 Hz"]} onChange={(value) => onChange({ inputNotchFilter: value })} />
          <SelectDraft label="动态储备" value={channel.dynamicReserve} options={["低噪声 (LNOise)", "正常 (NORMAL)", "高储备 (HIGH)"]} onChange={(value) => onChange({ dynamicReserve: value })} />
          <SelectDraft label="灵敏度" value={channel.sensitivity} options={["10 uV/nA", "30 uV/nA", "100 uV/nA", "300 uV/nA", "1 mV/nA", "3 mV/nA", "10 mV/nA", "30 mV/nA", "100 mV/nA", "300 mV/nA"]} onChange={(value) => onChange({ sensitivity: value })} />
          <DraftField label="滤波器时间常数 s" value={channel.timeConstantS} onChange={(value) => onChange({ timeConstantS: value })} />
          <SelectDraft label="滤波器陡降 dB/oct" value={channel.filterSlopeDbOct} options={["6", "12", "18", "24"]} onChange={(value) => onChange({ filterSlopeDbOct: value })} />
        </div>
      </details>
      <details open>
        <summary style={{ cursor: "pointer", fontWeight: 600 }}>参考信号</summary>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: "var(--space-2)", marginTop: "var(--space-2)" }}>
          <SelectDraft label="参考信号源" value={channel.referenceSource} options={["内部参考", "外部参考", "内部扫频"]} onChange={(value) => onChange({ referenceSource: value })} />
          <SelectDraft label="外部参考触发" value={channel.externalRefTrigger} options={["过零检测", "TTL 上升沿"]} onChange={(value) => onChange({ externalRefTrigger: value })} />
          <DisabledDraftField label="内部参考频率 Hz" value={channel.internalFrequencyHz} disabled={isExternal} reason="外部参考模式下内部频率不适用。" onChange={(value) => onChange({ internalFrequencyHz: value })} />
          <DraftField label="相位 deg" value={channel.phaseDeg} onChange={(value) => onChange({ phaseDeg: value })} />
          <DisabledDraftField label="内部扫频类型" value="线性扫频类型" disabled={channel.referenceSource !== "内部扫频"} reason="只有内部扫频模式才可编辑。" />
          <DisabledDraftField label="扫频运行模式" value="停止扫频" disabled={channel.referenceSource !== "内部扫频"} reason="内部扫频未激活。" />
        </div>
      </details>
      <details>
        <summary style={{ cursor: "pointer", fontWeight: 600 }}>通道输出 / 正弦输出</summary>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: "var(--space-2)", marginTop: "var(--space-2)" }}>
          <SelectDraft label="正弦输出模式" value={channel.sineoutMode} options={["固定幅值模式", "线性扫幅", "对数扫幅"]} onChange={(value) => onChange({ sineoutMode: value })} />
          <DraftField label="正弦幅值 Vrms" value={channel.sineoutVoltageVrms} onChange={(value) => onChange({ sineoutVoltageVrms: value })} />
          <DisabledDraftField label="扫幅开始 Vrms" value="1.000" disabled={isFixedSine} reason="固定幅值模式下扫幅参数不生效。" />
          <DisabledDraftField label="扫幅截止 Vrms" value="1.000" disabled={isFixedSine} reason="固定幅值模式下扫幅参数不生效。" />
          <DisabledDraftField label="直流输出 Vdc" value="0.000" disabled={isFixedSine} reason="固定幅值模式下直流输出幅值不适用。" />
          <SelectDraft label="通道输出源" value={channel.channelSource} options={["A-R", "X", "Y", "R", "Theta", "AUXOUT"]} onChange={(value) => onChange({ channelSource: value })} />
          <DraftField label="偏移 %" value={channel.offsetPercent} onChange={(value) => onChange({ offsetPercent: value })} />
          <DraftField label="放大" value={channel.expand} onChange={(value) => onChange({ expand: value })} />
          <SelectDraft label="速度" value={channel.speed} options={["慢速", "快速"]} onChange={(value) => onChange({ speed: value })} />
          <DisabledDraftField label="AUXOUT Vdc" value="0.000" disabled={channel.channelSource !== "AUXOUT"} reason="输出源不是 AUXOUT 时 AUXOUT 电压不适用。" />
        </div>
      </details>
    </div>
  );
}

function DisabledDraftField({
  label,
  value,
  disabled,
  reason,
  onChange,
}: {
  label: string;
  value: string;
  disabled: boolean;
  reason: string;
  onChange?: (value: string) => void;
}) {
  return (
    <label style={{ display: "grid", gap: 5, fontSize: "var(--font-size-xs)" }}>
      {label}
      <input value={value} disabled={disabled} readOnly={!onChange} onChange={(event) => onChange?.(event.target.value)} style={{ ...inputStyle, background: disabled ? "var(--color-disabled-bg)" : undefined, color: disabled ? "var(--color-disabled-text)" : undefined }} />
      {disabled && <span style={{ color: "var(--color-warning)" }}>{reason}</span>}
    </label>
  );
}

// ---------------------------------------------------------------------------
// Laser Card
// ---------------------------------------------------------------------------

function LaserCard({
  info,
  isConnected,
  address,
  identity,
  onAddressChange,
  onConnectChange,
  refreshTick,
}: {
  info: DeviceInfo;
  isConnected: boolean;
  address: string;
  identity?: string;
  onAddressChange: (v: string) => void;
  onConnectChange: () => void;
  refreshTick: number;
}) {
  const [status, setStatus] = useState<LaserStatus | null>(null);
  const [pwr, setPwr] = useState("");

  const { connecting, doConnect, doDisconnect, doIdentify } = useConnect(info, isConnected, address, onConnectChange);

  const fetchStatus = useCallback(async () => {
    try {
      const s: LaserStatus = await invoke("laser_get_status", { deviceId: info.id });
      setStatus(s);
    } catch {
      setStatus(null);
    }
  }, [info.id]);

  useEffect(() => {
    if (isConnected) fetchStatus();
  }, [fetchStatus, refreshTick, isConnected]);

  const cmd = async (name: string, args?: Record<string, unknown>) => {
    try {
      const s: LaserStatus = await invoke(name, { deviceId: info.id, ...args });
      setStatus(s);
    } catch (e) {
      alert(String(e));
    }
  };

  return (
    <div style={cardStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-2)" }}>
        <h2 style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>Laser</h2>
        <span style={isConnected ? badgeOk : badgeNeutral}>{isConnected ? "CONNECTED" : "DISCONNECTED"}</span>
      </div>

      <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center", marginBottom: "var(--space-3)" }}>
        <span style={{ ...smallMuted, whiteSpace: "nowrap" }}>Port:</span>
        <input type="text" value={address} onChange={(e) => onAddressChange(e.target.value)} style={inputStyle} disabled={isConnected} />
        {!isConnected ? (
          <button onClick={doConnect} disabled={connecting} style={btnPrimary}>{connecting ? "…" : "Connect"}</button>
        ) : (
          <button onClick={doDisconnect} style={btnDanger}>Disconnect</button>
        )}
        <button onClick={doIdentify} disabled={!isConnected} style={isConnected ? btnSecondary : btnDisabled}>Probe</button>
        <button onClick={fetchStatus} disabled={!isConnected} style={isConnected ? btnSecondary : btnDisabled}>Refresh</button>
      </div>

      <div style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-xs)", lineHeight: 1.6 }}>
        <div><strong>Identity:</strong> {identity ?? "No IDN (frame-echo protocol)"}</div>
        <div><strong>Last readback:</strong> {status?.last_command_time ?? "—"}</div>
      </div>

      {status && (
        <div style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-xs)", display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4 }}>
          <div><strong>Enabled:</strong> {status.enabled ? "ON" : "OFF"}</div>
          <div><strong>Power:</strong> {status.power_setpoint_mw} mW</div>
          <div><strong>Note:</strong> {status.note}</div>
        </div>
      )}

      {isConnected && (
        <div style={{ borderTop: "1px solid var(--color-border)", paddingTop: "var(--space-3)", display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
          <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
            <button onClick={() => cmd("laser_set_enabled", { enabled: false })} style={btnDanger}>Laser OFF</button>
            <button onClick={() => cmd("laser_set_power", { powerMw: 0 })} style={btnSecondary}>Set Power 0</button>
            <button onClick={() => cmd("laser_emergency_off")} style={{ ...btnDanger, background: "#7f1d1d" }}>Emergency OFF</button>
          </div>
          <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
            <input type="text" placeholder="Power (mW)" value={pwr} onChange={(e) => setPwr(e.target.value)} style={{ ...inputStyle, width: 90 }} />
            <button onClick={() => { const mw = parseInt(pwr, 10); if (!Number.isNaN(mw)) cmd("laser_set_power", { powerMw: mw }); }} style={btnSecondary}>Set Fixed Power</button>
            <button onClick={() => cmd("laser_set_enabled", { enabled: true })} style={btnPrimary}>Enable</button>
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Magnetic XYZ Package Table
// ---------------------------------------------------------------------------

function MagneticPackageTable({
  packageStatus,
  deviceInfos,
  connected,
  addresses,
  identities,
  onAddressChange,
  onChanged,
}: {
  packageStatus: MagneticXyzPackageStatus | null;
  deviceInfos: DeviceInfo[];
  connected: Set<string>;
  addresses: Record<string, string>;
  identities: Record<string, string>;
  onAddressChange: (deviceId: string, value: string) => void;
  onChanged: () => void;
}) {
  return (
    <div style={{ ...cardStyle, overflowX: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-3)" }}>
        <div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>Maynuo M8812 XYZ Axis Table</div>
          <div style={smallMuted}>Bx/By/Bz is the operator input; each row shows zero + recurrent current + readback reconstruction.</div>
        </div>
        <span style={packageStatus?.ready_to_apply ? badgeOk : badgeNeutral}>
          {packageStatus?.package_id ?? "maynuo_m8812_lab_xyz"}
        </span>
      </div>

      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-xs)", minWidth: 1320 }}>
        <thead>
          <tr style={{ color: "var(--color-text-muted)", textAlign: "left", borderBottom: "1px solid var(--color-border)" }}>
            <th style={{ padding: 8 }}>Axis</th>
            <th style={{ padding: 8 }}>Connection</th>
            <th style={{ padding: 8 }}>SN / IDN</th>
            <th style={{ padding: 8 }}>Coil</th>
            <th style={{ padding: 8 }}>Zero bias</th>
            <th style={{ padding: 8 }}>Zero readback</th>
            <th style={{ padding: 8 }}>Recur field</th>
            <th style={{ padding: 8 }}>Recur current</th>
            <th style={{ padding: 8 }}>Total command</th>
            <th style={{ padding: 8 }}>Total readback</th>
            <th style={{ padding: 8 }}>Reconstructed field</th>
            <th style={{ padding: 8 }}>Lock / Output</th>
            <th style={{ padding: 8 }}>Safety</th>
          </tr>
        </thead>
        <tbody>
          {deviceInfos.map((info) => {
            const row = packageStatus?.axes.find((axis) => axis.device_id === info.id);
            return (
              <MagneticPackageRow
                key={info.id}
                info={info}
                row={row}
                isConnected={connected.has(info.id)}
                address={addresses[info.id] ?? ""}
                identity={identities[info.id]}
                onAddressChange={(value) => onAddressChange(info.id, value)}
                onChanged={onChanged}
              />
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function MagneticPackageRow({
  info,
  row,
  isConnected,
  address,
  identity,
  onAddressChange,
  onChanged,
}: {
  info: DeviceInfo;
  row?: MagneticXyzPackageStatus["axes"][number];
  isConnected: boolean;
  address: string;
  identity?: string;
  onAddressChange: (value: string) => void;
  onChanged: () => void;
}) {
  const [bias, setBias] = useState("");
  const [recurMag, setRecurMag] = useState("");
  const { connecting, doConnect, doDisconnect, doIdentify } = useConnect(info, isConnected, address, onChanged);

  useEffect(() => {
    setBias(row ? row.zero_bias_a.toFixed(6) : "");
    setRecurMag(row ? row.recur_mag_nt.toFixed(3) : "");
  }, [row?.zero_bias_a, row?.recur_mag_nt]);

  const runMagCommand = async (name: string, args?: Record<string, unknown>) => {
    try {
      await invoke(name, { deviceId: info.id, ...args });
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  };

  const observed = row?.observed_idn ?? identity;
  const snMatch = row?.sn_match ?? (observed && info.expectedSn ? observed.includes(info.expectedSn) : undefined);
  const borderStyle = { borderBottom: "1px solid var(--color-border)" };
  const cellStyle: React.CSSProperties = { padding: 8, verticalAlign: "top", ...borderStyle };

  return (
    <tr>
      <td style={cellStyle}>
        <strong>{info.name.replace("Mag ", "")}</strong>
        <div style={smallMuted}>{info.id}</div>
      </td>
      <td style={cellStyle}>
        <div style={{ display: "grid", gap: 6 }}>
          <span style={isConnected ? badgeOk : badgeNeutral}>{isConnected ? "CONNECTED" : "DISCONNECTED"}</span>
          <input value={address} onChange={(e) => onAddressChange(e.target.value)} style={{ ...inputStyle, width: 150 }} disabled={isConnected} />
          <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
            {!isConnected ? (
              <button onClick={doConnect} disabled={connecting} style={btnPrimary}>{connecting ? "…" : "Connect"}</button>
            ) : (
              <button onClick={doDisconnect} style={btnDanger}>Disconnect</button>
            )}
            <button onClick={doIdentify} disabled={!isConnected} style={isConnected ? btnSecondary : btnDisabled}>Identify</button>
            <button onClick={() => onChanged()} style={btnSecondary}>Refresh</button>
          </div>
        </div>
      </td>
      <td style={cellStyle}>
        <div><strong>Expected:</strong> {row?.expected_sn ?? info.expectedSn ?? "—"}</div>
        <div style={{ maxWidth: 190, overflowWrap: "anywhere" }}><strong>IDN:</strong> {observed ?? "—"}</div>
        {snMatch !== undefined && <span style={snMatch ? badgeOk : badgeFail}>{snMatch ? "SN PASS" : "SN FAIL"}</span>}
      </td>
      <td style={cellStyle}>
        {row ? `${row.coil_constant_nt_per_ma.toFixed(2)} nT/mA` : "—"}
      </td>
      <td style={cellStyle}>
        <div>{fmtA(row?.zero_bias_a)}</div>
        <input value={bias} onChange={(e) => setBias(e.target.value)} style={{ ...inputStyle, width: 112, marginTop: 6 }} disabled={row?.lock_zero} />
        <button
          onClick={() => {
            const value = Number.parseFloat(bias);
            if (!Number.isNaN(value)) runMagCommand("magnetic_set_zero_bias", { biasA: value, outputOn: row?.output_on ?? false });
          }}
          disabled={!isConnected || row?.lock_zero}
          style={!isConnected || row?.lock_zero ? btnDisabled : btnSecondary}
        >
          Set A
        </button>
      </td>
      <td style={cellStyle}>
        {row?.runtime_zero_mean_a != null
          ? `${(row.runtime_zero_mean_a * 1000).toFixed(3)} ± ${((row.runtime_zero_std_a ?? 0) * 1000).toFixed(3)} mA`
          : "—"}
      </td>
      <td style={cellStyle}>
        <div>{row ? `${row.recur_mag_nt.toFixed(2)} nT` : "—"}</div>
        <input value={recurMag} onChange={(e) => setRecurMag(e.target.value)} style={{ ...inputStyle, width: 112, marginTop: 6 }} />
        <button
          onClick={() => {
            const value = Number.parseFloat(recurMag);
            if (!Number.isNaN(value)) runMagCommand("magnetic_set_recur_mag", { magNt: value, outputOn: row?.output_on ?? false });
          }}
          disabled={!isConnected}
          style={isConnected ? btnSecondary : btnDisabled}
        >
          Set nT
        </button>
      </td>
      <td style={cellStyle}>{fmtA(row?.recur_current_a)}</td>
      <td style={cellStyle}>{fmtA(row?.total_command_a)}</td>
      <td style={cellStyle}>{fmtA(row?.measured_total_current_a)}</td>
      <td style={cellStyle}>
        {row?.reconstructed_recur_mag_nt != null ? `${row.reconstructed_recur_mag_nt.toFixed(2)} nT` : "—"}
      </td>
      <td style={cellStyle}>
        <div style={{ display: "grid", gap: 6 }}>
          <span style={row?.lock_zero ? badgeOk : badgeNeutral}>{row?.lock_zero ? "ZERO LOCKED" : "ZERO UNLOCKED"}</span>
          <span style={row?.output_on ? badgeOk : badgeNeutral}>{row?.output_on ? "OUTPUT ON" : "OUTPUT OFF"}</span>
          <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
            <button onClick={() => runMagCommand("magnetic_toggle_lock_zero", { lock: true, outputOn: row?.output_on ?? false })} disabled={!isConnected} style={isConnected ? btnSecondary : btnDisabled}>Lock</button>
            <button onClick={() => runMagCommand("magnetic_toggle_lock_zero", { lock: false, outputOn: row?.output_on ?? false })} disabled={!isConnected} style={isConnected ? btnSecondary : btnDisabled}>Unlock</button>
            <button onClick={() => runMagCommand("magnetic_toggle_output", { on: !(row?.output_on ?? false) })} disabled={!isConnected} style={isConnected ? btnPrimary : btnDisabled}>{row?.output_on ? "OUT OFF" : "OUT ON"}</button>
          </div>
        </div>
      </td>
      <td style={cellStyle}>
        {row && row.blocked_reasons.length === 0 ? (
          <span style={badgeOk}>OK</span>
        ) : (
          <div style={{ color: "var(--color-danger)", maxWidth: 220 }}>
            {(row?.blocked_reasons ?? ["status unavailable"]).join(" · ")}
          </div>
        )}
        <button onClick={() => runMagCommand("magnetic_safe_cleanup")} disabled={!isConnected} style={{ ...(isConnected ? btnDanger : btnDisabled), marginTop: 6 }}>Cleanup</button>
      </td>
    </tr>
  );
}

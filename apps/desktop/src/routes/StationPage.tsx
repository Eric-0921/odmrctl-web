import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { StationProfile, StationPreflightReport, WorkbenchSnapshot } from "../types/deviceWorkbench";

type PageState =
  | { tag: "empty" }
  | { tag: "profile_loaded"; profile: StationProfile; path: string }
  | { tag: "preflight_running"; profile: StationProfile; path: string }
  | { tag: "preflight_done"; profile: StationProfile; path: string; report: StationPreflightReport }
  | { tag: "error"; message: string; previous?: PageState };

const cardStyle: React.CSSProperties = {
  background: "var(--color-surface)",
  border: "1px solid var(--color-border)",
  borderRadius: "var(--radius-md)",
  padding: "var(--space-4)",
};

const badge = (ok: boolean): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 10px",
  borderRadius: "var(--radius-sm)",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  background: ok ? "var(--color-success-soft)" : "var(--color-danger-soft)",
  color: ok ? "var(--color-success)" : "var(--color-danger)",
});

const btnPrimary: React.CSSProperties = {
  padding: "8px 16px",
  borderRadius: "var(--radius-sm)",
  border: "none",
  background: "var(--color-primary)",
  color: "#fff",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  cursor: "pointer",
};

const btnSecondary: React.CSSProperties = {
  padding: "8px 16px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  color: "var(--color-text)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  cursor: "pointer",
};

const btnDanger: React.CSSProperties = {
  padding: "8px 16px",
  borderRadius: "var(--radius-sm)",
  border: "none",
  background: "var(--color-danger)",
  color: "#fff",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  cursor: "pointer",
};

const codeBlockStyle: React.CSSProperties = {
  background: "#1e1e2e",
  color: "#cdd6f4",
  borderRadius: "var(--radius-sm)",
  padding: "var(--space-3)",
  fontSize: "var(--font-size-xs)",
  fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
  overflowX: "auto" as const,
  lineHeight: 1.5,
};

const stepNumberStyle: React.CSSProperties = {
  width: 24,
  height: 24,
  borderRadius: "50%",
  background: "var(--color-primary)",
  color: "#fff",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontSize: "var(--font-size-xs)",
  fontWeight: 700,
  flexShrink: 0,
};

const exampleJson = `{
  "name": "My Lab Station",
  "devices": [
    {
      "device_id": "smb100a_main",
      "kind": "rf_source",
      "transport": "tcp_raw_socket",
      "address": "192.168.1.20:5025"
    },
    {
      "device_id": "oe1022d_main",
      "kind": "lock_in",
      "transport": "uart",
      "address": "/dev/ttyUSB0"
    },
    {
      "device_id": "maynuo.mag_x",
      "kind": "magnetic",
      "transport": "uart",
      "address": "auto",
      "expected_sn": "2020"
    }
  ],
  "safety": {
    "smb100a_max_power_dbm": 0.0,
    "smb100a_max_freq_hz": 3000000000,
    "mag_max_current_a_per_axis": 0.1,
    "laser_max_power_mw": 100
  }
}`;

export default function StationPage() {
  const [state, setState] = useState<PageState>({ tag: "empty" });
  const [snapshot, setSnapshot] = useState<WorkbenchSnapshot | null>(null);
  const [operatorApproved, setOperatorApproved] = useState(false);
  const [showExample, setShowExample] = useState(false);

  const refreshSnapshot = async () => {
    try {
      const snap: WorkbenchSnapshot = await invoke("get_workbench_state");
      setSnapshot(snap);
      // Sync page state with backend state on first load
      if (state.tag === "empty" && snap.profile_loaded && snap.report) {
        setState({
          tag: "preflight_done",
          profile: { name: snap.profile_name ?? "Unknown", devices: [] },
          path: "",
          report: snap.report,
        });
      } else if (state.tag === "empty" && snap.profile_loaded) {
        setState({
          tag: "profile_loaded",
          profile: { name: snap.profile_name ?? "Unknown", devices: [] },
          path: "",
        });
      }
    } catch {
      setSnapshot(null);
    }
  };

  useEffect(() => {
    refreshSnapshot();
    const interval = setInterval(refreshSnapshot, 2000);
    return () => clearInterval(interval);
  }, []);

  const handlePickProfile = async () => {
    try {
      const path = (await invoke("pick_recipe_file")) as string | null;
      if (!path) return;
      const profile: StationProfile = await invoke("load_station_profile", { path });
      setState({ tag: "profile_loaded", profile, path });
      await refreshSnapshot();
    } catch (e) {
      setState({ tag: "error", message: String(e) });
    }
  };

  const handleLoadExample = async () => {
    try {
      const profile: StationProfile = await invoke("load_example_station_profile");
      setState({ tag: "profile_loaded", profile, path: "(built-in example)" });
      await refreshSnapshot();
    } catch (e) {
      setState({ tag: "error", message: String(e) });
    }
  };

  const handleRunPreflight = async () => {
    if (state.tag !== "profile_loaded") return;
    const { path, profile } = state;
    setState({ tag: "preflight_running", profile, path });
    try {
      const report: StationPreflightReport = await invoke("run_station_preflight_cmd", {
        operatorApproved: operatorApproved,
      });
      setState({ tag: "preflight_done", profile, path, report });
      await refreshSnapshot();
    } catch (e) {
      setState({ tag: "error", message: String(e), previous: state });
    }
  };

  const handleReleaseLocks = async () => {
    try {
      await invoke("release_all_locks");
      await refreshSnapshot();
      setState({ tag: "empty" });
    } catch (e) {
      setState({ tag: "error", message: String(e) });
    }
  };

  const overallPassed = (report: StationPreflightReport) =>
    report.all_devices_reachable && report.all_identities_verified && report.all_safe_states_confirmed;

  const isProfileLoaded = state.tag === "profile_loaded" || state.tag === "preflight_running" || state.tag === "preflight_done";

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700, marginBottom: "var(--space-4)" }}>
        Station Workbench
      </h1>

      {/* --- Workbench state banner --- */}
      {snapshot && (
        <div
          style={{
            ...cardStyle,
            marginBottom: "var(--space-4)",
            display: "flex",
            gap: "var(--space-3)",
            alignItems: "center",
            flexWrap: "wrap",
          }}
        >
          <span style={badge(snapshot.preflight_passed)}>
            {snapshot.preflight_passed ? "LOCKED" : "UNLOCKED"}
          </span>
          <span style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)" }}>
            Profile: {snapshot.profile_name ?? "none"} · Locks:{" "}
            {snapshot.locks_held.length > 0 ? snapshot.locks_held.join(", ") : "none"}
          </span>
          {snapshot.preflight_passed && (
            <button onClick={handleReleaseLocks} style={btnDanger}>
              Release Locks
            </button>
          )}
        </div>
      )}

      {/* --- Empty state: Getting Started guide --- */}
      {!isProfileLoaded && (
        <div style={{ ...cardStyle, marginBottom: "var(--space-4)" }}>
          <div
            style={{
              fontSize: "var(--font-size-lg)",
              fontWeight: 600,
              marginBottom: "var(--space-3)",
            }}
          >
            Getting Started
          </div>

          <p style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-4)", lineHeight: 1.6 }}>
            Before you can use the device panels (SMB100A, OE1022D, Magnetic, Laser), you need to
            tell the app which devices are connected and how to reach them. This is done through a{" "}
            <strong>station profile</strong> — a JSON file that lists every device with its
            connection address (IP or COM/tty port) and safety limits.
          </p>

          {/* Steps */}
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)", marginBottom: "var(--space-4)" }}>
            <div style={{ display: "flex", gap: "var(--space-3)", alignItems: "flex-start" }}>
              <span style={stepNumberStyle}>1</span>
              <div>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-sm)" }}>Load a station profile</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)" }}>
                  Browse for your <code>station.json</code>, or load the built-in example to explore
                  the UI.
                </div>
              </div>
            </div>
            <div style={{ display: "flex", gap: "var(--space-3)", alignItems: "flex-start" }}>
              <span style={stepNumberStyle}>2</span>
              <div>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-sm)" }}>Run preflight</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)" }}>
                  The app connects to each device, verifies its identity, checks safe states, and
                  acquires exclusive locks.
                </div>
              </div>
            </div>
            <div style={{ display: "flex", gap: "var(--space-3)", alignItems: "flex-start" }}>
              <span style={stepNumberStyle}>3</span>
              <div>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-sm)" }}>Use device panels</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)" }}>
                  Once preflight passes, the SMB100A / OE1022D / Magnetic / Laser panels unlock and
                  you can read status and send commands.
                </div>
              </div>
            </div>
          </div>

          {/* Action buttons */}
          <div style={{ display: "flex", gap: "var(--space-3)", flexWrap: "wrap", marginBottom: "var(--space-3)" }}>
            <button onClick={handlePickProfile} style={btnPrimary}>
              Browse for station.json…
            </button>
            <button onClick={handleLoadExample} style={btnSecondary}>
              Load Example Profile
            </button>
            <button
              onClick={() => setShowExample((s) => !s)}
              style={{ ...btnSecondary, border: "none", textDecoration: "underline" }}
            >
              {showExample ? "Hide Example" : "Show Example JSON"}
            </button>
          </div>

          {/* Example JSON */}
          {showExample && (
            <div style={{ marginTop: "var(--space-2)" }}>
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)", marginBottom: "var(--space-2)" }}>
                Example <code>station.json</code> — save this as a file and edit the addresses for your lab:
              </div>
              <pre style={codeBlockStyle}>{exampleJson}</pre>
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)", marginTop: "var(--space-2)" }}>
                There is also a full example at{" "}
                <code>examples/stations/odmr_station.full.example.json</code> in the project repo.
              </div>
            </div>
          )}
        </div>
      )}

      {/* --- Actions row (shown when profile loaded) --- */}
      {isProfileLoaded && (
        <div style={{ display: "flex", gap: "var(--space-3)", alignItems: "center", marginBottom: "var(--space-4)", flexWrap: "wrap" }}>
          <button onClick={handlePickProfile} style={btnSecondary}>
            Change Profile…
          </button>
          {(state.tag === "profile_loaded" || state.tag === "preflight_running" || state.tag === "preflight_done") && (
            <>
              <button
                onClick={handleRunPreflight}
                disabled={state.tag === "preflight_running"}
                style={{
                  ...btnPrimary,
                  background: "var(--color-accent)",
                  opacity: state.tag === "preflight_running" ? 0.6 : 1,
                }}
              >
                {state.tag === "preflight_running" ? "Preflight Running…" : "Run Preflight"}
              </button>
              <label
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "var(--space-2)",
                  fontSize: "var(--font-size-sm)",
                }}
              >
                <input
                  type="checkbox"
                  checked={operatorApproved}
                  onChange={(e) => setOperatorApproved(e.target.checked)}
                />
                Operator approved
              </label>
            </>
          )}
        </div>
      )}

      {/* --- Error --- */}
      {state.tag === "error" && (
        <div
          style={{
            background: "var(--color-danger-soft)",
            border: "1px solid var(--color-danger)",
            borderRadius: "var(--radius-md)",
            padding: "var(--space-4)",
            marginBottom: "var(--space-4)",
            color: "var(--color-danger)",
          }}
        >
          <strong>Error:</strong> {state.message}
        </div>
      )}

      {/* --- Profile summary --- */}
      {(state.tag === "profile_loaded" || state.tag === "preflight_running" || state.tag === "preflight_done") && (
        <div style={{ ...cardStyle, marginBottom: "var(--space-4)" }}>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: "var(--space-2)" }}>
            Profile: {state.profile.name}
          </div>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-3)" }}>
            Path: {state.path}
          </div>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-sm)" }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--color-border)" }}>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Device ID</th>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Kind</th>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Transport</th>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Address</th>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Expected S/N</th>
              </tr>
            </thead>
            <tbody>
              {state.profile.devices.map((dev) => (
                <tr key={dev.device_id} style={{ borderBottom: "1px solid var(--color-border-subtle)" }}>
                  <td style={{ padding: "6px 8px" }}>{dev.device_id}</td>
                  <td style={{ padding: "6px 8px" }}>{dev.kind}</td>
                  <td style={{ padding: "6px 8px" }}>{dev.transport}</td>
                  <td style={{ padding: "6px 8px" }}>{dev.address ?? "—"}</td>
                  <td style={{ padding: "6px 8px" }}>{dev.expected_sn ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* --- Preflight report --- */}
      {state.tag === "preflight_done" && (
        <>
          {/* Overall banner */}
          <div
            style={{
              ...cardStyle,
              marginBottom: "var(--space-4)",
              background: overallPassed(state.report)
                ? "var(--color-success-soft)"
                : "var(--color-danger-soft)",
              borderColor: overallPassed(state.report)
                ? "var(--color-success)"
                : "var(--color-danger)",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "var(--space-3)",
                marginBottom: "var(--space-2)",
              }}
            >
              <span style={badge(overallPassed(state.report))}>
                {overallPassed(state.report) ? "PASS" : "FAIL"}
              </span>
              <span style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>
                Preflight {overallPassed(state.report) ? "Passed" : "Failed"}
              </span>
            </div>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)" }}>
              Elapsed: {state.report.elapsed_ms} ms · Operator approved:{" "}
              {state.report.operator_approved ? "yes" : "no"}
            </div>
          </div>

          {/* Device report table */}
          <div style={{ ...cardStyle, marginBottom: "var(--space-4)" }}>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: "var(--space-3)" }}>
              Device Reports
            </div>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-sm)" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--color-border)" }}>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Device</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Reachable</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Identity</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Safe State</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Errors</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Warnings</th>
                </tr>
              </thead>
              <tbody>
                {state.report.devices.map((d) => (
                  <tr key={d.device_id} style={{ borderBottom: "1px solid var(--color-border-subtle)" }}>
                    <td style={{ padding: "6px 8px" }}>
                      <div style={{ fontWeight: 600 }}>{d.device_id}</div>
                      <div style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)" }}>
                        {d.kind}
                      </div>
                    </td>
                    <td style={{ padding: "6px 8px" }}>
                      <span style={badge(d.reachability)}>{d.reachability ? "YES" : "NO"}</span>
                    </td>
                    <td style={{ padding: "6px 8px" }}>
                      {d.identity_display ? (
                        <span style={badge(true)}>OK</span>
                      ) : (
                        <span style={badge(false)}>FAIL</span>
                      )}
                      <div
                        style={{
                          fontSize: "var(--font-size-xs)",
                          color: "var(--color-text-muted)",
                          marginTop: 2,
                        }}
                      >
                        {d.identity_display ?? d.identity_raw ?? "—"}
                      </div>
                    </td>
                    <td style={{ padding: "6px 8px" }}>
                      {d.safe_state ? (
                        <span style={badge(d.safe_state.confirmed)}>
                          {d.safe_state.confirmed ? "CONFIRMED" : "FAIL"}
                        </span>
                      ) : (
                        <span style={badge(false)}>N/A</span>
                      )}
                    </td>
                    <td style={{ padding: "6px 8px" }}>
                      {d.error_queue.length > 0 ? (
                        <div style={{ color: "var(--color-danger)", fontSize: "var(--font-size-xs)" }}>
                          {d.error_queue.join(", ")}
                        </div>
                      ) : (
                        <span style={{ color: "var(--color-text-muted)" }}>—</span>
                      )}
                    </td>
                    <td style={{ padding: "6px 8px" }}>
                      {d.warnings.length > 0 ? (
                        <div style={{ color: "var(--color-warning)", fontSize: "var(--font-size-xs)" }}>
                          {d.warnings.join(", ")}
                        </div>
                      ) : (
                        <span style={{ color: "var(--color-text-muted)" }}>—</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Lock status */}
          <div style={{ ...cardStyle, marginBottom: "var(--space-4)" }}>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: "var(--space-3)" }}>
              Lock Status
            </div>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-sm)" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--color-border)" }}>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Device</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Acquired</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Lock File</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>PID</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Error</th>
                </tr>
              </thead>
              <tbody>
                {state.report.lock_status.map((ls) => (
                  <tr key={ls.device_id} style={{ borderBottom: "1px solid var(--color-border-subtle)" }}>
                    <td style={{ padding: "6px 8px" }}>{ls.device_id}</td>
                    <td style={{ padding: "6px 8px" }}>
                      <span style={badge(ls.acquired)}>{ls.acquired ? "YES" : "NO"}</span>
                    </td>
                    <td
                      style={{
                        padding: "6px 8px",
                        fontSize: "var(--font-size-xs)",
                        color: "var(--color-text-muted)",
                      }}
                    >
                      {ls.lock_file}
                    </td>
                    <td style={{ padding: "6px 8px" }}>{ls.pid ?? "—"}</td>
                    <td style={{ padding: "6px 8px", color: "var(--color-danger)", fontSize: "var(--font-size-xs)" }}>
                      {ls.error ?? "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}

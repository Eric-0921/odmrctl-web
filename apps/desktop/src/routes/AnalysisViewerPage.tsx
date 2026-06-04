import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import type { AnalysisData } from "../types/analysis";
import { formatVoltage, pickVoltageUnit } from "../utils/formatVoltage";

type LoadState =
  | { tag: "empty" }
  | { tag: "loading" }
  | { tag: "error"; message: string }
  | { tag: "loaded"; data: AnalysisData };

const cardStyle: React.CSSProperties = {
  background: "var(--color-surface)",
  border: "1px solid var(--color-border)",
  borderRadius: "var(--radius-md)",
  padding: "var(--space-3)",
};

const badge = (ok: boolean): React.CSSProperties => ({
  display: "inline-block",
  padding: "1px 8px",
  borderRadius: "var(--radius-sm)",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  background: ok ? "var(--color-success-soft)" : "var(--color-danger-soft)",
  color: ok ? "var(--color-success)" : "var(--color-danger)",
});

const sectionTitle: React.CSSProperties = {
  fontSize: "var(--font-size-lg)",
  fontWeight: 600,
  marginBottom: "var(--space-3)",
  color: "var(--color-text)",
};

export default function AnalysisViewerPage() {
  const [state, setState] = useState<LoadState>({ tag: "empty" });
  const [dirPath, setDirPath] = useState<string>("");

  const handlePick = async () => {
    try {
      const picked: string | null = await invoke("pick_analysis_directory");
      if (!picked) return;
      setDirPath(picked);
      setState({ tag: "loading" });
      const data: AnalysisData = await invoke("read_analysis_directory", {
        path: picked,
      });
      setState({ tag: "loaded", data });
    } catch (e) {
      setState({ tag: "error", message: String(e) });
    }
  };

  const ghz = (hz: number) => (hz / 1e9).toFixed(4);

  // --- Empty state ---
  if (state.tag === "empty") {
    return (
      <div style={{ padding: "var(--space-6)" }}>
        <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700, marginBottom: "var(--space-4)" }}>
          M4.0 Read-only Analysis Viewer
        </h1>
        <div style={{ ...cardStyle, textAlign: "center", padding: "var(--space-8)", maxWidth: 560, margin: "0 auto" }}>
          <p style={{ color: "var(--color-text-muted)", marginBottom: "var(--space-4)" }}>
            Select an M3.6 analysis directory to view results.
          </p>
          <button
            onClick={handlePick}
            style={{
              padding: "10px 24px",
              fontSize: "var(--font-size-base)",
              fontWeight: 600,
              background: "var(--color-primary)",
              color: "#fff",
              border: "none",
              borderRadius: "var(--radius-md)",
              cursor: "pointer",
            }}
          >
            Select Analysis Directory
          </button>
        </div>
      </div>
    );
  }

  // --- Loading state ---
  if (state.tag === "loading") {
    return (
      <div style={{ padding: "var(--space-6)" }}>
        <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700 }}>M4.0 Read-only Analysis Viewer</h1>
        <p style={{ color: "var(--color-text-muted)", marginTop: "var(--space-4)" }}>
          Loading analysis from {dirPath}...
        </p>
      </div>
    );
  }

  // --- Error state ---
  if (state.tag === "error") {
    return (
      <div style={{ padding: "var(--space-6)" }}>
        <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700 }}>M4.0 Read-only Analysis Viewer</h1>
        <div
          style={{
            ...cardStyle,
            borderColor: "var(--color-danger)",
            background: "var(--color-danger-soft)",
            marginTop: "var(--space-4)",
            maxWidth: 640,
          }}
        >
          <p style={{ fontWeight: 600, color: "var(--color-danger)", marginBottom: "var(--space-2)" }}>
            Error loading analysis from:
          </p>
          <p style={{ color: "var(--color-text-muted)", marginBottom: "var(--space-3)", wordBreak: "break-all" }}>
            {dirPath}
          </p>
          <pre style={{ fontSize: "var(--font-size-sm)", whiteSpace: "pre-wrap" }}>{state.message}</pre>
          <button
            onClick={() => setState({ tag: "empty" })}
            style={{
              marginTop: "var(--space-3)",
              padding: "6px 16px",
              background: "var(--color-primary)",
              color: "#fff",
              border: "none",
              borderRadius: "var(--radius-sm)",
              cursor: "pointer",
              fontWeight: 600,
            }}
          >
            Try Again
          </button>
        </div>
      </div>
    );
  }

  // --- Loaded state ---
  const { data } = state;
  const qf = data.quality_flags;
  const as_ = data.analysis_summary;
  const ro = data.run_overlay_summary;

  // Pick a unified display unit for all voltage values in this analysis
  const allBxValues = ro.frequencies.map((f) => f.b_x_mean_mv);
  const allByValues = ro.frequencies.map((f) => f.b_y_mean_mv);
  const displayUnit = pickVoltageUnit([...allBxValues, ...allByValues]);

  const chartData = ro.frequencies.map((f) => ({
    freq_ghz: f.frequency_hz / 1e9,
    bxMean: f.b_x_mean_mv,
    byMean: f.b_y_mean_mv,
  }));

  return (
    <div style={{ padding: "var(--space-6)", paddingBottom: "var(--space-2)" }}>
      {/* 1. Header */}
      <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700, marginBottom: "var(--space-1)" }}>
        M4.0 Read-only Analysis Viewer
      </h1>
      <p style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-sm)", marginBottom: "var(--space-4)", wordBreak: "break-all" }}>
        {dirPath}
      </p>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "var(--space-3)", marginBottom: "var(--space-4)" }}>
        <div style={cardStyle}>
          <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)" }}>Quality Grade</div>
          <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700 }}>
            <span style={badge(qf.passed)}>{qf.passed ? "PASSED" : "FAILED"}</span>
          </div>
        </div>
        <div style={cardStyle}>
          <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)" }}>ODMR Dip</div>
          <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, color: as_.odmr_dip_detected ? "var(--color-accent)" : "var(--color-text-muted)" }}>
            {as_.odmr_dip_detected ? "DETECTED" : "None"}
          </div>
        </div>
        <div style={cardStyle}>
          <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)" }}>Physical Response Required</div>
          <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700 }}>
            {as_.physical_odmr_response_required ? "YES" : "No"}
          </div>
        </div>
        <div style={cardStyle}>
          <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)" }}>Input Runs</div>
          <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700 }}>{as_.source_run_ids.length}</div>
        </div>
      </div>

      {/* 2. Quality Flags Panel */}
      <h2 style={sectionTitle}>Quality Flags</h2>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "var(--space-2)", marginBottom: "var(--space-4)" }}>
        {[
          ["All Runs Passed", !qf.failed_run],
          ["All Safe States Confirmed", !qf.unsafe_final_state],
          ["No Parse Failures", !qf.parse_failures],
          ["Audit Clean", !qf.audit_mismatch],
          ["No CSV", !qf.csv_present],
          ["No Magnetic", !qf.magnetic_command_present],
          ["Frequency Grid Match", !qf.frequency_grid_mismatch],
          ["No Missing Artifacts", !qf.missing_artifact],
          ["Signal Present", !qf.empty_signal_series],
        ].map(([label, ok]) => (
          <div key={label as string} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontSize: "var(--font-size-sm)" }}>{label as string}</span>
              <span style={badge(ok as boolean)}>{(ok as boolean) ? "PASS" : "FAIL"}</span>
            </div>
          </div>
        ))}
      </div>
      {qf.parse_failure_count > 0 && (
        <p style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-3)" }}>
          Parse failures: {qf.parse_failure_count} (rate: {as_.parse_failure_rate.toFixed(4)})
        </p>
      )}
      {data.warnings.length > 0 && (
        <div style={{ ...cardStyle, borderColor: "var(--color-warning)", marginBottom: "var(--space-4)" }}>
          <p style={{ fontWeight: 600, color: "var(--color-warning)" }}>Warnings</p>
          {data.warnings.map((w, i) => (
            <p key={i} style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)" }}>{w}</p>
          ))}
        </div>
      )}

      {/* 3. Spectrum Plot */}
      <h2 style={sectionTitle}>Frequency vs Signal</h2>
      <div style={{ ...cardStyle, marginBottom: "var(--space-4)", height: 360 }}>
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData} margin={{ top: 16, right: 24, left: 16, bottom: 16 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
            <XAxis
              dataKey="freq_ghz"
              label={{ value: "Frequency (GHz)", position: "insideBottom", offset: -8, style: { fill: "var(--color-text-muted)", fontSize: 12 } }}
              tick={{ fontSize: 11, fill: "var(--color-text-muted)" }}
            />
            <YAxis
              label={{ value: `Signal (${displayUnit})`, angle: -90, position: "insideLeft", offset: 8, style: { fill: "var(--color-text-muted)", fontSize: 12 } }}
              tick={{ fontSize: 11, fill: "var(--color-text-muted)" }}
            />
            <Tooltip
              contentStyle={{
                background: "var(--color-surface)",
                border: "1px solid var(--color-border)",
                borderRadius: "var(--radius-sm)",
                fontSize: "var(--font-size-sm)",
              }}
              formatter={(value, name) => [
                formatVoltage(Number(value), { unit: displayUnit, digits: 4 }),
                String(name),
              ]}
              labelFormatter={(label) => `${Number(label).toFixed(4)} GHz`}
            />
            <Legend wrapperStyle={{ fontSize: "var(--font-size-sm)" }} />
            <Line type="monotone" dataKey="bxMean" name="B-X mean" stroke="var(--color-primary)" dot={{ r: 2 }} strokeWidth={2} />
            <Line type="monotone" dataKey="byMean" name="B-Y mean" stroke="var(--color-accent)" dot={{ r: 2 }} strokeWidth={2} />
          </LineChart>
        </ResponsiveContainer>
      </div>
      <p style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)", marginTop: "-12px", marginBottom: "var(--space-4)" }}>
        Flat signal does not indicate ODMR resonance. ODMR dip detection requires further analysis.
      </p>

      {/* 4. Run Overlay Summary */}
      <h2 style={sectionTitle}>Run Overlay Summary ({ro.frequency_count} frequencies)</h2>
      <div style={{ overflowX: "auto", marginBottom: "var(--space-4)" }}>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)" }}>
          <thead>
            <tr>
              <th>Freq (GHz)</th>
              <th>B-X Mean ({displayUnit})</th>
              <th>B-X Std ({displayUnit})</th>
              <th>B-Y Mean ({displayUnit})</th>
              <th>B-Y Std ({displayUnit})</th>
              <th>Points</th>
              <th>Frames</th>
            </tr>
          </thead>
          <tbody>
            {ro.frequencies.map((f) => (
              <tr key={f.frequency_hz}>
                <td>{ghz(f.frequency_hz)}</td>
                <td>{formatVoltage(f.b_x_mean_mv, { unit: displayUnit, digits: 3 })}</td>
                <td>{formatVoltage(f.b_x_std_mv, { unit: displayUnit, digits: 3 })}</td>
                <td>{formatVoltage(f.b_y_mean_mv, { unit: displayUnit, digits: 3 })}</td>
                <td>{formatVoltage(f.b_y_std_mv, { unit: displayUnit, digits: 3 })}</td>
                <td>{f.point_count}</td>
                <td>{f.total_frames_used}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* 5. Source Runs */}
      <h2 style={sectionTitle}>Source Runs ({as_.source_run_ids.length})</h2>
      <div style={{ overflowX: "auto", marginBottom: "var(--space-4)" }}>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)" }}>
          <thead>
            <tr>
              <th>Run ID</th>
              <th>OE1022D Identity</th>
            </tr>
          </thead>
          <tbody>
            {as_.source_run_ids.map((rid) => (
              <tr key={rid}>
                <td style={{ fontSize: "var(--font-size-xs)" }}>{rid}</td>
                <td style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)", maxWidth: 320, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {as_.oe1022d_display_idn_by_run[rid] ?? "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* 6. Analysis Summary */}
      <h2 style={sectionTitle}>Analysis Summary</h2>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "var(--space-2)", marginBottom: "var(--space-4)" }}>
        {[
          ["Frequency Range", ro.frequencies.length > 0 ? `${ghz(ro.frequencies[0].frequency_hz)} – ${ghz(ro.frequencies[ro.frequencies.length - 1].frequency_hz)} GHz` : "N/A"],
          ["Frequency Points", String(as_.frequency_count)],
          ["Total Frames Used", String(as_.frames_used)],
          ["Frames Parse Failed", String(as_.frames_parse_failed)],
          ["Parse Failure Rate", as_.parse_failure_rate.toFixed(4)],
          ["B-X Contrast", as_.contrast_estimate_b_x_mv != null ? formatVoltage(as_.contrast_estimate_b_x_mv, { unit: displayUnit, digits: 3 }) : "N/A"],
          ["B-Y Contrast", as_.contrast_estimate_b_y_mv != null ? formatVoltage(as_.contrast_estimate_b_y_mv, { unit: displayUnit, digits: 3 }) : "N/A"],
          ["Quality Grade", qf.passed ? "PASSED" : "FAILED"],
          ["ODMR Dip Claimed", as_.odmr_dip_detected ? "YES" : "NO"],
        ].map(([label, value]) => (
          <div key={label as string} style={cardStyle}>
            <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)" }}>{label as string}</div>
            <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600 }}>{value as string}</div>
          </div>
        ))}
      </div>

      {/* 7. Boundary Banner */}
      <div
        style={{
          background: "var(--color-primary-soft)",
          borderLeft: "4px solid var(--color-primary)",
          padding: "var(--space-3) var(--space-4)",
          fontSize: "var(--font-size-sm)",
          color: "var(--color-primary-strong)",
          fontWeight: 500,
          borderRadius: "var(--radius-sm)",
          marginTop: "var(--space-4)",
        }}
      >
        M4.0 READ-ONLY VIEWER — No hardware connection. No recipe execution. No magnetic control.
      </div>
    </div>
  );
}

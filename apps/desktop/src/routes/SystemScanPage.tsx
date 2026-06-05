import { useState } from "react";
import {
  m5bRecipe,
  m5bResolved,
  m5bSafetyReport,
  m5bDryRunPlan,
  m5bStation,
  m5bDeviceProfiles,
} from "../mock-data/m5b";

type TabKey =
  | "overview"
  | "recipe"
  | "station-safety"
  | "device-profiles"
  | "resolved-steps"
  | "safety-report"
  | "dry-run";

const tabs: { key: TabKey; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "recipe", label: "Recipe" },
  { key: "station-safety", label: "Station Safety" },
  { key: "device-profiles", label: "Device Profiles" },
  { key: "resolved-steps", label: "Resolved Steps" },
  { key: "safety-report", label: "Safety Report" },
  { key: "dry-run", label: "Dry Run" },
];

function statusStyle(status: string) {
  switch (status) {
    case "pass":
      return { bg: "var(--color-success-soft)", color: "var(--color-success)" };
    case "warn":
      return { bg: "var(--color-warning-soft)", color: "var(--color-warning)" };
    case "fail":
      return { bg: "var(--color-danger-soft)", color: "var(--color-danger)" };
    default:
      return { bg: "var(--color-disabled-bg)", color: "var(--color-disabled-text)" };
  }
}

function phaseStyle(phase: string) {
  switch (phase) {
    case "setup":
      return { bg: "var(--color-primary-soft)", color: "var(--color-primary)" };
    case "measure":
      return { bg: "var(--color-accent-soft)", color: "var(--color-accent)" };
    case "cleanup":
      return { bg: "var(--color-warning-soft)", color: "var(--color-warning)" };
    default:
      return { bg: "var(--color-disabled-bg)", color: "var(--color-disabled-text)" };
  }
}

function renderJsonTree(value: unknown, depth = 0): React.ReactNode {
  if (value === null) return <span style={{ color: "var(--color-text-muted)" }}>null</span>;
  if (typeof value === "boolean") return <span style={{ color: "var(--color-primary)" }}>{String(value)}</span>;
  if (typeof value === "number") return <span style={{ fontFamily: "var(--font-mono)" }}>{value}</span>;
  if (typeof value === "string") return <span style={{ color: "var(--color-success)" }}>{value}</span>;
  if (Array.isArray(value)) {
    if (value.length === 0) return <span>[]</span>;
    const isMatrix =
      value.length === 3 &&
      Array.isArray(value[0]) &&
      Array.isArray(value[1]) &&
      Array.isArray(value[2]);
    if (isMatrix) {
      return (
        <div style={{ display: "grid", gap: "var(--space-1)", marginLeft: depth > 0 ? 16 : 0 }}>
          {value.map((row, i) => (
            <div key={i} style={{ display: "flex", gap: "var(--space-2)", fontFamily: "var(--font-mono)" }}>
              {Array.isArray(row) ? row.map((cell, j) => (
                <span key={j} style={{ minWidth: 80, textAlign: "right" }}>{typeof cell === "number" ? cell.toExponential(4) : String(cell)}</span>
              )) : String(row)}
            </div>
          ))}
        </div>
      );
    }
    if (value.length > 0 && typeof value[0] === "number" && value.length <= 6) {
      return (
        <span style={{ fontFamily: "var(--font-mono)" }}>
          [{value.map((v, i) => (
            <span key={i}>{typeof v === "number" ? v.toLocaleString() : String(v)}{i < value.length - 1 ? ", " : ""}</span>
          ))}]
        </span>
      );
    }
    return (
      <div style={{ marginLeft: depth > 0 ? 16 : 0 }}>
        {value.map((v, i) => (
          <div key={i} style={{ marginBottom: 2 }}>
            {renderJsonTree(v, depth + 1)}
          </div>
        ))}
      </div>
    );
  }
  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return <span>{}</span>;
    return (
      <div style={{ marginLeft: depth > 0 ? 16 : 0 }}>
        {entries.map(([k, v]) => (
          <div key={k} style={{ marginBottom: "var(--space-1)" }}>
            <span style={{ fontWeight: 600, color: "var(--color-text)" }}>{k}</span>
            <span style={{ margin: "0 4px", color: "var(--color-text-muted)" }}>:</span>
            {renderJsonTree(v, depth + 1)}
          </div>
        ))}
      </div>
    );
  }
  return <span>{String(value)}</span>;
}

// ---------------------------------------------------------------------------
// Sub-page components
// ---------------------------------------------------------------------------

function OverviewTab() {
  const recipe = m5bRecipe;
  const resolved = m5bResolved;
  const safety = m5bSafetyReport;
  const dryRun = m5bDryRunPlan;

  const sweepPoints = recipe.sweeps.reduce((acc, s) => {
    if (s.type === "cartesian_grid" && s.axes) {
      const dims = Object.values(s.axes).map((a) => (a.values ? a.values.length : 1));
      return acc * dims.reduce((p, d) => p * d, 1);
    }
    if (s.values) return acc * s.values.length;
    return acc;
  }, 1);

  const decisionColor = safety.decision === "allow" ? "var(--color-success)" : "var(--color-danger)";
  const decisionBg = safety.decision === "allow" ? "var(--color-success-soft)" : "var(--color-danger-soft)";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
      <div
        style={{
          background: decisionBg,
          borderLeft: `4px solid ${decisionColor}`,
          padding: "var(--space-4)",
          borderRadius: "0 var(--radius-md) var(--radius-md) 0",
        }}
      >
        <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 600, color: decisionColor }}>
          Safety decision: {safety.decision.toUpperCase()}
        </div>
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginTop: "var(--space-2)" }}>
          {safety.requires_operator_approval ? "Operator approval required" : "No operator approval required"} ·{" "}
          {safety.summary.warning_count} warnings · {safety.summary.error_count} errors
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "var(--space-4)" }}>
        {[
          { title: "Recipe ID", value: recipe.id },
          { title: "Total steps", value: String(resolved.step_count) },
          { title: "Sweep points", value: String(sweepPoints) },
          { title: "Estimated duration", value: `${dryRun.summary.estimated_duration_s} s` },
          { title: "Outer sweep", value: dryRun.summary.outer_sweep },
          { title: "Inner sweep", value: dryRun.summary.inner_sweep },
          { title: "Expected frames", value: String(dryRun.summary.expected_frames) },
          { title: "Required devices", value: dryRun.summary.required_devices.join(", ") },
        ].map((card) => (
          <div
            key={card.title}
            style={{
              background: "var(--color-surface)",
              borderRadius: "var(--radius-md)",
              padding: "var(--space-4)",
              boxShadow: "var(--shadow-card)",
              border: "1px solid var(--color-border)",
            }}
          >
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-2)" }}>
              {card.title}
            </div>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, wordBreak: "break-word" }}>{card.value}</div>
          </div>
        ))}
      </div>

      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
        }}
      >
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>Sweep dimensions</h3>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              {["Sweep ID", "Device", "Type", "Unit", "Dimensions"].map((h) => (
                <th key={h} style={{ padding: "var(--table-density-cell-padding)", textAlign: "left", fontWeight: 600, borderBottom: "1px solid var(--color-border)" }}>
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {recipe.sweeps.map((s) => {
              let dims = "—";
              if (s.type === "cartesian_grid" && s.axes) {
                const axes = Object.entries(s.axes);
                dims = axes.map(([k, v]) => `${k}: ${v.values ? v.values.length : 1} pts`).join(" × ");
              } else if (s.values) {
                dims = `${s.values.length} pts`;
              }
              return (
                <tr key={s.sweep_id} style={{ borderBottom: "1px solid var(--color-border)" }}>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{s.sweep_id}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{s.device || "—"}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{s.type || "list"}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{s.unit || "—"}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{dims}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
        }}
      >
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>Safety summary</h3>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "var(--space-4)" }}>
          {[
            { title: "Checked steps", value: String(safety.summary.checked_steps) },
            { title: "Checked actions", value: String(safety.summary.checked_actions) },
            { title: "Info / Warning / Error", value: `${safety.summary.info_count} / ${safety.summary.warning_count} / ${safety.summary.error_count}` },
            { title: "Operator approval", value: safety.requires_operator_approval ? "Required" : "Not required" },
          ].map((card) => (
            <div
              key={card.title}
              style={{
                background: "var(--color-bg)",
                borderRadius: "var(--radius-sm)",
                padding: "var(--space-3)",
                border: "1px solid var(--color-border)",
              }}
            >
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
                {card.title}
              </div>
              <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>{card.value}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function RecipeTab() {
  const recipe = m5bRecipe;
  const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({
    devices: true,
    fixed_params: true,
    sweeps: true,
    acquisition_policy: true,
    safety: true,
  });

  const toggle = (key: string) =>
    setExpandedSections((prev) => ({ ...prev, [key]: !prev[key] }));

  const section = (title: string, key: string, content: React.ReactNode) => (
    <div
      key={key}
      style={{
        background: "var(--color-surface)",
        borderRadius: "var(--radius-md)",
        border: "1px solid var(--color-border)",
        marginBottom: "var(--space-4)",
        overflow: "hidden",
      }}
    >
      <button
        onClick={() => toggle(key)}
        style={{
          width: "100%",
          padding: "var(--space-3) var(--space-4)",
          textAlign: "left",
          background: "transparent",
          border: "none",
          borderBottom: expandedSections[key] ? "1px solid var(--color-border)" : "none",
          cursor: "pointer",
          fontSize: "var(--font-size-md)",
          fontWeight: 600,
          color: "var(--color-text)",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <span>{title}</span>
        <span style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-sm)" }}>
          {expandedSections[key] ? "▼" : "▶"}
        </span>
      </button>
      {expandedSections[key] && <div style={{ padding: "var(--space-4)" }}>{content}</div>}
    </div>
  );

  return (
    <div>
      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
          marginBottom: "var(--space-6)",
        }}
      >
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>Recipe metadata</h3>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <tbody>
            {[
              { key: "ID", value: recipe.id },
              { key: "Kind", value: recipe.kind },
              { key: "Schema version", value: recipe.schema_version },
              { key: "Description", value: recipe.description },
              { key: "Station ref", value: recipe.station_ref },
              { key: "Physical response required", value: String(recipe.physical_response_required) },
            ].map((row) => (
              <tr key={row.key} style={{ borderBottom: "1px solid var(--color-border)" }}>
                <td style={{ padding: "8px 0", color: "var(--color-text-muted)", width: "30%" }}>{row.key}</td>
                <td style={{ padding: "8px 0", fontFamily: "var(--font-mono)" }}>{row.value}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {section("Devices", "devices", renderJsonTree(recipe.devices))}
      {section("Fixed params", "fixed_params", renderJsonTree(recipe.fixed_params))}
      {section("Sweeps", "sweeps", renderJsonTree(recipe.sweeps))}
      {section("Acquisition policy", "acquisition_policy", renderJsonTree(recipe.acquisition_policy))}
      {section("Safety", "safety", renderJsonTree(recipe.safety))}
    </div>
  );
}

function StationSafetyTab() {
  const station = m5bStation;
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
        }}
      >
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>Station metadata</h3>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <tbody>
            {[
              { key: "ID", value: station.id },
              { key: "Name", value: station.name },
              { key: "Description", value: station.description },
              { key: "Schema version", value: station.schema_version },
            ].map((row) => (
              <tr key={row.key} style={{ borderBottom: "1px solid var(--color-border)" }}>
                <td style={{ padding: "8px 0", color: "var(--color-text-muted)", width: "30%" }}>{row.key}</td>
                <td style={{ padding: "8px 0", fontFamily: "var(--font-mono)" }}>{row.value}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
        }}
      >
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>Devices</h3>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              {["Device ID", "Kind", "Transport", "Address", "Expected S/N", "Timeout (ms)", "Profile"].map((h) => (
                <th key={h} style={{ padding: "var(--table-density-cell-padding)", textAlign: "left", fontWeight: 600, borderBottom: "1px solid var(--color-border)" }}>
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {station.devices.map((d) => (
              <tr key={d.device_id} style={{ borderBottom: "1px solid var(--color-border)" }}>
                <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{d.device_id}</td>
                <td style={{ padding: "var(--table-density-cell-padding)" }}>{d.kind}</td>
                <td style={{ padding: "var(--table-density-cell-padding)" }}>{d.transport}</td>
                <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{d.address ?? "—"}</td>
                <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{d.expected_sn ?? "—"}</td>
                <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{d.timeout_ms}</td>
                <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{d.profile_ref ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
        }}
      >
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>Safety limits</h3>
        {renderJsonTree(station.safety)}
      </div>
    </div>
  );
}

function DeviceProfilesTab() {
  const [activeProfile, setActiveProfile] = useState(0);
  const profiles = m5bDeviceProfiles;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
      <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
        {profiles.map((p, i) => (
          <button
            key={p.id}
            onClick={() => setActiveProfile(i)}
            style={{
              padding: "8px 16px",
              borderRadius: "var(--radius-sm)",
              border: "1px solid var(--color-border)",
              background: i === activeProfile ? "var(--color-primary-soft)" : "var(--color-surface)",
              color: i === activeProfile ? "var(--color-primary)" : "var(--color-text)",
              cursor: "pointer",
              fontSize: "var(--font-size-sm)",
              fontWeight: i === activeProfile ? 600 : 400,
            }}
          >
            {p.device_type} — {p.id}
          </button>
        ))}
      </div>

      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
        }}
      >
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>
          {profiles[activeProfile].device_type} profile
        </h3>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse", marginBottom: "var(--space-4)" }}>
          <tbody>
            {[
              { key: "ID", value: profiles[activeProfile].id },
              { key: "Device type", value: profiles[activeProfile].device_type },
              { key: "Description", value: profiles[activeProfile].description },
              { key: "Schema version", value: profiles[activeProfile].schema_version },
            ].map((row) => (
              <tr key={row.key} style={{ borderBottom: "1px solid var(--color-border)" }}>
                <td style={{ padding: "8px 0", color: "var(--color-text-muted)", width: "20%" }}>{row.key}</td>
                <td style={{ padding: "8px 0", fontFamily: "var(--font-mono)" }}>{row.value}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {renderJsonTree(profiles[activeProfile])}
      </div>
    </div>
  );
}

function ResolvedStepsTab() {
  const resolved = m5bResolved;
  const [selectedStepId, setSelectedStepId] = useState<string | null>(null);

  const selectedStep = resolved.steps.find((s) => s.step_id === selectedStepId);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
        }}
      >
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>Resolved steps ({resolved.step_count})</h3>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              {["Step ID", "Phase", "Point", "Sweep coordinates", "Acquisition"].map((h) => (
                <th key={h} style={{ padding: "var(--table-density-cell-padding)", textAlign: "left", fontWeight: 600, borderBottom: "1px solid var(--color-border)" }}>
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {resolved.steps.map((step) => {
              const pStyle = phaseStyle(step.phase);
              return (
                <tr
                  key={step.step_id}
                  onClick={() => setSelectedStepId(step.step_id)}
                  style={{
                    borderBottom: "1px solid var(--color-border)",
                    cursor: "pointer",
                    background: selectedStepId === step.step_id ? "var(--color-primary-soft)" : "transparent",
                  }}
                >
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{step.step_id}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>
                    <span
                      style={{
                        fontSize: "var(--font-size-xs)",
                        padding: "2px 8px",
                        borderRadius: "var(--radius-sm)",
                        background: pStyle.bg,
                        color: pStyle.color,
                        fontWeight: 600,
                        textTransform: "uppercase",
                      }}
                    >
                      {step.phase}
                    </span>
                  </td>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>
                    {step.point_index !== undefined ? String(step.point_index) : "—"}
                  </td>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)" }}>
                    {step.sweep_coordinates
                      ? Object.entries(step.sweep_coordinates)
                          .map(([k, v]) => `${k}=${typeof v === "number" ? v.toLocaleString() : v}`)
                          .join(", ")
                      : "—"}
                  </td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>
                    {step.acquisition.enabled ? (
                      <span
                        style={{
                          fontSize: "var(--font-size-xs)",
                          padding: "2px 8px",
                          borderRadius: "var(--radius-sm)",
                          background: "var(--color-accent-soft)",
                          color: "var(--color-accent)",
                          fontWeight: 600,
                        }}
                      >
                        ON
                      </span>
                    ) : (
                      <span
                        style={{
                          fontSize: "var(--font-size-xs)",
                          padding: "2px 8px",
                          borderRadius: "var(--radius-sm)",
                          background: "var(--color-disabled-bg)",
                          color: "var(--color-disabled-text)",
                          fontWeight: 600,
                        }}
                      >
                        OFF
                      </span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {selectedStep && (
        <div
          style={{
            background: "var(--color-surface)",
            borderRadius: "var(--radius-md)",
            padding: "var(--space-4)",
            border: "1px solid var(--color-border)",
          }}
        >
          <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>
            Step detail: {selectedStep.step_id}
          </h3>
          {renderJsonTree(selectedStep)}
        </div>
      )}
    </div>
  );
}

function SafetyReportTab() {
  const report = m5bSafetyReport;
  const [filter, setFilter] = useState<"all" | "pass" | "warn" | "fail">("all");

  const filtered =
    filter === "all" ? report.checks : report.checks.filter((c) => c.status === filter);

  const decisionColor = report.decision === "allow" ? "var(--color-success)" : "var(--color-danger)";
  const decisionBg = report.decision === "allow" ? "var(--color-success-soft)" : "var(--color-danger-soft)";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
      <div
        style={{
          background: decisionBg,
          borderLeft: `4px solid ${decisionColor}`,
          padding: "var(--space-4)",
          borderRadius: "0 var(--radius-md) var(--radius-md) 0",
        }}
      >
        <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 600, color: decisionColor }}>
          Decision: {report.decision.toUpperCase()}
        </div>
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginTop: "var(--space-2)" }}>
          {report.checks.length} checks evaluated · {report.summary.warning_count} warnings ·{" "}
          {report.summary.error_count} errors
        </div>
      </div>

      <div style={{ display: "flex", gap: "var(--space-2)" }}>
        {(["all", "pass", "warn", "fail"] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            style={{
              padding: "6px 14px",
              borderRadius: "var(--radius-sm)",
              border: "1px solid var(--color-border)",
              background: filter === f ? "var(--color-primary-soft)" : "var(--color-surface)",
              color: filter === f ? "var(--color-primary)" : "var(--color-text)",
              cursor: "pointer",
              fontSize: "var(--font-size-sm)",
              fontWeight: filter === f ? 600 : 400,
              textTransform: "capitalize",
            }}
          >
            {f} ({f === "all" ? report.checks.length : report.checks.filter((c) => c.status === f).length})
          </button>
        ))}
      </div>

      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
        }}
      >
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              {["Status", "Check", "Message", "Value", "Limit"].map((h) => (
                <th key={h} style={{ padding: "var(--table-density-cell-padding)", textAlign: "left", fontWeight: 600, borderBottom: "1px solid var(--color-border)" }}>
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {filtered.map((check, i) => {
              const s = statusStyle(check.status);
              return (
                <tr key={i} style={{ borderBottom: "1px solid var(--color-border)" }}>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>
                    <span
                      style={{
                        fontSize: "var(--font-size-xs)",
                        padding: "2px 8px",
                        borderRadius: "var(--radius-sm)",
                        background: s.bg,
                        color: s.color,
                        fontWeight: 600,
                        textTransform: "uppercase",
                      }}
                    >
                      {check.status}
                    </span>
                  </td>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)" }}>
                    {check.check}
                  </td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{check.message}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>
                    {check.value !== undefined ? (typeof check.value === "number" && Math.abs(check.value) < 0.001 ? check.value.toExponential(2) : String(check.value)) : "—"}
                  </td>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>
                    {check.limit !== undefined ? String(check.limit) : "—"}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function DryRunTab() {
  const plan = m5bDryRunPlan;
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "var(--space-4)" }}>
        {[
          { title: "Total steps", value: String(plan.summary.step_count) },
          { title: "Total points", value: String(plan.summary.total_points) },
          { title: "Expected frames", value: String(plan.summary.expected_frames) },
          { title: "Estimated duration", value: `${plan.summary.estimated_duration_s} s` },
          { title: "Outer sweep", value: plan.summary.outer_sweep },
          { title: "Inner sweep", value: plan.summary.inner_sweep },
          { title: "Hazard actions", value: String(plan.summary.hazard_actions) },
          { title: "Operator approval", value: plan.operator_approval_required ? "Required" : "Not required" },
        ].map((card) => (
          <div
            key={card.title}
            style={{
              background: "var(--color-surface)",
              borderRadius: "var(--radius-md)",
              padding: "var(--space-4)",
              boxShadow: "var(--shadow-card)",
              border: "1px solid var(--color-border)",
            }}
          >
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-2)" }}>
              {card.title}
            </div>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, wordBreak: "break-word" }}>{card.value}</div>
          </div>
        ))}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
        {plan.phases.map((phase) => {
          const pStyle = phaseStyle(phase.phase);
          return (
            <div
              key={phase.phase}
              style={{
                background: "var(--color-surface)",
                borderRadius: "var(--radius-md)",
                border: "1px solid var(--color-border)",
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  padding: "var(--space-3) var(--space-4)",
                  borderBottom: "1px solid var(--color-border)",
                  display: "flex",
                  alignItems: "center",
                  gap: "var(--space-3)",
                }}
              >
                <span
                  style={{
                    fontSize: "var(--font-size-xs)",
                    padding: "2px 10px",
                    borderRadius: "var(--radius-sm)",
                    background: pStyle.bg,
                    color: pStyle.color,
                    fontWeight: 600,
                    textTransform: "uppercase",
                  }}
                >
                  {phase.phase}
                </span>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{phase.description}</span>
                {phase.hazard_note && (
                  <span
                    style={{
                      fontSize: "var(--font-size-xs)",
                      padding: "2px 8px",
                      borderRadius: "var(--radius-sm)",
                      background: "var(--color-warning-soft)",
                      color: "var(--color-warning)",
                      fontWeight: 600,
                      marginLeft: "auto",
                    }}
                  >
                    {phase.hazard_note}
                  </span>
                )}
              </div>
              <div style={{ padding: "var(--space-4)" }}>
                <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
                  <thead>
                    <tr>
                      {["Step ID", "Description"].map((h) => (
                        <th key={h} style={{ padding: "var(--table-density-cell-padding)", textAlign: "left", fontWeight: 600, borderBottom: "1px solid var(--color-border)" }}>
                          {h}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {phase.steps.map((step) => (
                      <tr key={step.step_id} style={{ borderBottom: "1px solid var(--color-border)" }}>
                        <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)", width: "25%" }}>
                          {step.step_id}
                        </td>
                        <td style={{ padding: "var(--table-density-cell-padding)" }}>{step.description}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export default function SystemScanPage() {
  const [activeTab, setActiveTab] = useState<TabKey>("overview");

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", marginBottom: "var(--space-4)" }}>
        <h1 style={{ fontSize: "var(--font-size-2xl)", margin: 0 }}>System Scan</h1>
        <span
          style={{
            fontSize: "var(--font-size-xs)",
            padding: "2px 10px",
            borderRadius: "var(--radius-sm)",
            background: "var(--color-primary-soft)",
            color: "var(--color-primary)",
            fontWeight: 600,
            textTransform: "uppercase",
          }}
        >
          M5B-B
        </span>
        <span
          style={{
            fontSize: "var(--font-size-xs)",
            padding: "2px 10px",
            borderRadius: "var(--radius-sm)",
            background: "var(--color-warning-soft)",
            color: "var(--color-warning)",
            fontWeight: 600,
            marginLeft: "auto",
          }}
        >
          READ ONLY
        </span>
      </div>

      <div
        style={{
          display: "flex",
          gap: "var(--space-1)",
          borderBottom: "1px solid var(--color-border)",
          marginBottom: "var(--space-6)",
          overflow: "auto",
        }}
      >
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setActiveTab(t.key)}
            style={{
              padding: "10px 16px",
              border: "none",
              borderBottom: activeTab === t.key ? "3px solid var(--color-primary)" : "3px solid transparent",
              background: "transparent",
              color: activeTab === t.key ? "var(--color-primary)" : "var(--color-text-muted)",
              cursor: "pointer",
              fontSize: "var(--font-size-sm)",
              fontWeight: activeTab === t.key ? 600 : 400,
              whiteSpace: "nowrap",
            }}
          >
            {t.label}
          </button>
        ))}
      </div>

      {activeTab === "overview" && <OverviewTab />}
      {activeTab === "recipe" && <RecipeTab />}
      {activeTab === "station-safety" && <StationSafetyTab />}
      {activeTab === "device-profiles" && <DeviceProfilesTab />}
      {activeTab === "resolved-steps" && <ResolvedStepsTab />}
      {activeTab === "safety-report" && <SafetyReportTab />}
      {activeTab === "dry-run" && <DryRunTab />}
    </div>
  );
}

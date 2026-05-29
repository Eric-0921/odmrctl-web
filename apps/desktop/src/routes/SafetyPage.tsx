import { getSafetyReport } from "../mock-data/helpers";

function severityStyle(severity: string) {
  switch (severity) {
    case "info":
      return { bg: "var(--color-success-soft)", color: "var(--color-success)" };
    case "warning":
      return { bg: "var(--color-warning-soft)", color: "var(--color-warning)" };
    case "error":
      return { bg: "var(--color-danger-soft)", color: "var(--color-danger)" };
    default:
      return { bg: "var(--color-disabled-bg)", color: "var(--color-disabled-text)" };
  }
}

export default function SafetyPage() {
  const report = getSafetyReport();
  const decisionColor = report.decision === "allow" ? "var(--color-success)" : "var(--color-danger)";
  const decisionBg = report.decision === "allow" ? "var(--color-success-soft)" : "var(--color-danger-soft)";

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        Safety
      </h1>

      <div
        style={{
          background: decisionBg,
          borderLeft: `4px solid ${decisionColor}`,
          padding: "var(--space-4)",
          marginBottom: "var(--space-6)",
          borderRadius: "0 var(--radius-md) var(--radius-md) 0",
        }}
      >
        <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 600, color: decisionColor }}>
          Safety decision: {report.decision.charAt(0).toUpperCase() + report.decision.slice(1)}
        </div>
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginTop: "var(--space-2)" }}>
          Displayed from existing mock safety report. Frontend does not compute safety.
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gap: "var(--space-4)",
          marginBottom: "var(--space-6)",
        }}
      >
        {[
          { title: "Checked steps", value: String(report.summary.checked_steps) },
          { title: "Checked actions", value: String(report.summary.checked_actions) },
          { title: "Info / Warning / Error", value: `${report.summary.info_count} / ${report.summary.warning_count} / ${report.summary.error_count}` },
          { title: "Source", value: "basic_odmr_mock.safety_report.json" },
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
            <div
              style={{
                fontSize: "var(--font-size-sm)",
                color: "var(--color-text-muted)",
                marginBottom: "var(--space-2)",
              }}
            >
              {card.title}
            </div>
            <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 600 }}>
              {card.value}
            </div>
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
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>
          Findings
        </h3>
        {report.findings.length === 0 ? (
          <p style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-sm)" }}>
            No safety findings are available in this mock report.
          </p>
        ) : (
          <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
            <thead>
              <tr>
                {["severity", "code", "message", "step_id", "device"].map((h) => (
                  <th key={h} scope="col" style={{ padding: "var(--table-density-cell-padding)", textAlign: "left", fontWeight: 600, borderBottom: "1px solid var(--color-border)" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {report.findings.map((finding, i) => {
                const sStyle = severityStyle(finding.severity);
                return (
                  <tr key={i}>
                    <td style={{ padding: "var(--table-density-cell-padding)" }}>
                      <span
                        style={{
                          fontSize: "var(--font-size-xs)",
                          padding: "2px 8px",
                          borderRadius: "var(--radius-sm)",
                          background: sStyle.bg,
                          color: sStyle.color,
                          fontWeight: 600,
                        }}
                      >
                        {finding.severity}
                      </span>
                    </td>
                    <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{finding.code || "—"}</td>
                    <td style={{ padding: "var(--table-density-cell-padding)" }}>{finding.message}</td>
                    <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{finding.step_id || "—"}</td>
                    <td style={{ padding: "var(--table-density-cell-padding)" }}>{finding.device_id || "—"}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

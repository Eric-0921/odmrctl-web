import { getSafetyReport } from "../mock-data/helpers";

export default function SafetyPage() {
  const report = getSafetyReport();

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        Safety
      </h1>

      <div
        style={{
          background: report.decision === "allow" ? "var(--color-success-soft)" : "var(--color-danger-soft)",
          borderLeft: `4px solid ${report.decision === "allow" ? "var(--color-success)" : "var(--color-danger)"}`,
          padding: "var(--space-4)",
          marginBottom: "var(--space-6)",
          borderRadius: "0 var(--radius-md) var(--radius-md) 0",
        }}
      >
        <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 600, color: report.decision === "allow" ? "var(--color-success)" : "var(--color-danger)" }}>
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
              <tr style={{ background: "var(--color-surface-alt)" }}>
                {["severity", "code", "message", "step_id", "device"].map((h) => (
                  <th key={h} style={{ padding: "10px 12px", textAlign: "left", fontWeight: 600, borderBottom: "1px solid var(--color-border)" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {report.findings.map((finding, i) => (
                <tr key={i} style={{ borderBottom: "1px solid var(--color-border)" }}>
                  <td style={{ padding: "8px 12px" }}>{finding.severity}</td>
                  <td style={{ padding: "8px 12px", fontFamily: "var(--font-mono)" }}>{finding.code || "—"}</td>
                  <td style={{ padding: "8px 12px" }}>{finding.message}</td>
                  <td style={{ padding: "8px 12px", fontFamily: "var(--font-mono)" }}>{finding.step_id || "—"}</td>
                  <td style={{ padding: "8px 12px" }}>{finding.device_id || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

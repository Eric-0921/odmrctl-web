import { getRunSummary } from "../mock-data/helpers";

export default function DashboardPage() {
  const summary = getRunSummary();

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        Dashboard
      </h1>
      <p style={{ color: "var(--color-text-muted)", marginBottom: "var(--space-6)" }}>
        Mock run overview and operator state summary.
      </p>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(3, 1fr)",
          gap: "var(--space-4)",
        }}
      >
        {[
          { title: "System phase", value: "M1 mock complete / M2 pending" },
          { title: "Current mode", value: "GUI-M0 MOCK ONLY" },
          { title: "Safety decision", value: summary.safetyDecision },
          { title: "Resolved steps", value: String(summary.stepCount) },
          { title: "Estimated duration", value: `${summary.estimatedDurationMs / 1000} s` },
          { title: "Event count", value: String(summary.eventCount) },
          { title: "Artifact count", value: String(summary.artifactCount) },
          { title: "Backend state", value: "Static mock data" },
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
          marginTop: "var(--space-6)",
          display: "flex",
          gap: "var(--space-3)",
          flexWrap: "wrap",
        }}
      >
        {[
          { label: "Start Run", reason: "Requires executor backend" },
          { label: "Pause Run", reason: "Requires active executor run" },
          { label: "Stop Run", reason: "Requires active executor run" },
          { label: "Emergency Stop", reason: "M0 has no hardware authority" },
        ].map((btn) => (
          <div key={btn.label} style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
            <button
              disabled
              style={{
                padding: "8px 16px",
                borderRadius: "var(--radius-sm)",
                border: "1px solid var(--color-border)",
                background: "var(--color-disabled-bg)",
                color: "var(--color-disabled-text)",
                cursor: "not-allowed",
                fontSize: "var(--font-size-sm)",
              }}
            >
              {btn.label}
            </button>
            <span style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-subtle)" }}>
              {btn.reason}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

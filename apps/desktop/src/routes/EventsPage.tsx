import { getEvents, getRunSummary } from "../mock-data/helpers";

function levelStyle(level: string) {
  switch (level) {
    case "info":
      return { bg: "var(--color-success-soft)", color: "var(--color-success)" };
    case "warning":
      return { bg: "var(--color-warning-soft)", color: "var(--color-warning)" };
    case "error":
    case "danger":
      return { bg: "var(--color-danger-soft)", color: "var(--color-danger)" };
    default:
      return { bg: "var(--color-disabled-bg)", color: "var(--color-disabled-text)" };
  }
}

export default function EventsPage() {
  const events = getEvents();
  const summary = getRunSummary();

  const displayEvents = events.slice(0, 10);

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        Events
      </h1>
      <div
        style={{
          display: "flex",
          gap: "var(--space-4)",
          marginBottom: "var(--space-6)",
        }}
      >
        {[
          { title: "Event count", value: String(events.length) },
          { title: "Source", value: "events.jsonl" },
          { title: "Run ID", value: summary.runId },
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
          border: "1px solid var(--color-border)",
          overflow: "hidden",
        }}
      >
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              {["timestamp", "event_type", "step_id", "level", "message"].map((h) => (
                <th
                  key={h}
                  style={{
                    padding: "var(--table-density-cell-padding)",
                    textAlign: "left",
                    fontWeight: 600,
                    borderBottom: "1px solid var(--color-border)",
                  }}
                >
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {displayEvents.map((evt, i) => {
              const lStyle = levelStyle(evt.level);
              return (
                <tr key={i}>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)" }}>
                    {new Date(evt.timestamp_unix_ms).toISOString()}
                  </td>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{evt.event_type}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{evt.step_id || "—"}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>
                    <span
                      style={{
                        fontSize: "var(--font-size-xs)",
                        padding: "2px 8px",
                        borderRadius: "var(--radius-sm)",
                        background: lStyle.bg,
                        color: lStyle.color,
                        fontWeight: 600,
                      }}
                    >
                      {evt.level}
                    </span>
                  </td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{evt.message}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
        <div
          style={{
            padding: "var(--space-3)",
            textAlign: "center",
            fontSize: "var(--font-size-sm)",
            color: "var(--color-text-muted)",
            borderTop: "1px solid var(--color-border)",
          }}
        >
          Showing {displayEvents.length} of {events.length} events. Full event log is available in mock data.
        </div>
      </div>
    </div>
  );
}

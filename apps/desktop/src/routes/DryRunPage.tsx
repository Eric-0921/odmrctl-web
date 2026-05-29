import { getDryRunPlan, getDryRunSteps } from "../mock-data/helpers";

function parseDeviceAction(action: string): { device: string; actionName: string } {
  const parts = action.split(": ");
  return {
    device: parts[0] || "—",
    actionName: parts[1] || action,
  };
}

export default function DryRunPage() {
  const plan = getDryRunPlan();
  const steps = getDryRunSteps();

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        Dry Run
      </h1>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gap: "var(--space-4)",
          marginBottom: "var(--space-6)",
        }}
      >
        {[
          { title: "Total steps", value: String(plan.summary.step_count) },
          { title: "Estimated duration", value: `${plan.summary.estimated_duration_s} s` },
          { title: "Required devices", value: plan.summary.required_devices.join(", ") },
          { title: "Gated actions", value: String(plan.summary.hazard_actions) },
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
              {["#", "step_id", "device", "action", "parameters", "duration", "safety"].map((h) => (
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
            {steps.slice(0, 50).map((step, idx) => {
              const { device, actionName } = parseDeviceAction(step.device_actions[0] || "");
              return (
                <tr key={step.step_id}>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{idx + 1}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)" }}>{step.step_id}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{device}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{actionName}</td>
                  <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)" }}>
                    {Object.entries(step.sweep_coordinate).map(([k, v]) => (
                      <span key={k}>{k}={v.toLocaleString()}</span>
                    ))}
                  </td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>{step.estimated_duration_ms} ms</td>
                  <td style={{ padding: "var(--table-density-cell-padding)" }}>
                    <span
                      style={{
                        fontSize: "var(--font-size-xs)",
                        padding: "2px 8px",
                        borderRadius: "var(--radius-sm)",
                        background: "var(--color-success-soft)",
                        color: "var(--color-success)",
                      }}
                    >
                      safe
                    </span>
                  </td>
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
          Showing 50 of {plan.summary.step_count} steps. Full dry-run plan is available in mock data.
        </div>
      </div>
    </div>
  );
}

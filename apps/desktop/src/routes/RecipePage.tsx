import { useState } from "react";
import { getRecipe, getRunSummary } from "../mock-data/helpers";

export default function RecipePage() {
  const recipe = getRecipe();
  const summary = getRunSummary();
  const [showJson, setShowJson] = useState(false);

  const sweep = recipe.sweeps[0];

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        Recipe
      </h1>

      {/* Summary cards */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gap: "var(--space-4)",
          marginBottom: "var(--space-6)",
        }}
      >
        {[
          { title: "Recipe name", value: recipe.name },
          { title: "Schema version", value: recipe.schema_version },
          { title: "Resolved steps", value: String(summary.stepCount) },
          { title: "Required devices", value: summary.requiredDevices.join(", ") },
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
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>
              {card.value}
            </div>
          </div>
        ))}
      </div>

      {/* Sweep parameters */}
      {sweep && (
        <div
          style={{
            background: "var(--color-surface)",
            borderRadius: "var(--radius-md)",
            padding: "var(--space-4)",
            border: "1px solid var(--color-border)",
            marginBottom: "var(--space-6)",
          }}
        >
          <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>
            Main sweep parameters
          </h3>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(4, 1fr)",
              gap: "var(--space-4)",
            }}
          >
            {[
              { label: "Axis", value: sweep.axis },
              { label: "Start", value: `${(sweep.start / 1e9).toFixed(3)} GHz` },
              { label: "Stop", value: `${(sweep.stop / 1e9).toFixed(3)} GHz` },
              { label: "Step", value: `${(sweep.step / 1e6).toFixed(1)} MHz` },
              { label: "Order", value: sweep.order },
              { label: "Points", value: String(Math.round((sweep.stop - sweep.start) / sweep.step) + 1) },
            ].map((item) => (
              <div key={item.label}>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)" }}>
                  {item.label}
                </div>
                <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, fontFamily: "var(--font-mono)" }}>
                  {item.value}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Metadata table */}
      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
          marginBottom: "var(--space-6)",
        }}
      >
        <h3 style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-lg)" }}>
          Recipe metadata
        </h3>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <tbody>
            {[
              { key: "Recipe ID", value: recipe.id },
              { key: "Recipe hash", value: summary.recipeHash.slice(0, 8) + "..." + summary.recipeHash.slice(-6) },
              { key: "Schema version", value: recipe.schema_version },
              { key: "Station / profile", value: recipe.station_id },
              { key: "Experiment type", value: recipe.intent.experiment_type },
              { key: "Description", value: recipe.intent.description },
            ].map((row) => (
              <tr key={row.key} style={{ borderBottom: "1px solid var(--color-border)" }}>
                <td style={{ padding: "8px 0", color: "var(--color-text-muted)", width: "30%" }}>{row.key}</td>
                <td style={{ padding: "8px 0", fontFamily: "var(--font-mono)" }}>{row.value}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Read-only JSON preview */}
      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          border: "1px solid var(--color-border)",
          marginBottom: "var(--space-6)",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            padding: "var(--space-3) var(--space-4)",
            borderBottom: showJson ? "1px solid var(--color-border)" : "none",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <h3 style={{ fontSize: "var(--font-size-lg)" }}>Recipe JSON preview</h3>
          <button
            onClick={() => setShowJson((s) => !s)}
            aria-expanded={showJson}
            style={{
              padding: "4px 12px",
              borderRadius: "var(--radius-sm)",
              border: "1px solid var(--color-border)",
              background: "var(--color-surface-alt)",
              color: "var(--color-text)",
              cursor: "pointer",
              fontSize: "var(--font-size-sm)",
            }}
          >
            {showJson ? "Hide JSON" : "Show JSON"}
          </button>
        </div>
        {showJson && (
          <pre
            style={{
              padding: "var(--space-4)",
              fontSize: "var(--font-size-xs)",
              overflow: "auto",
              maxHeight: 400,
              background: "var(--color-bg)",
              margin: 0,
            }}
          >
            {JSON.stringify(recipe, null, 2)}
          </pre>
        )}
      </div>

      {/* Disabled controls */}
      <div style={{ display: "flex", gap: "var(--space-3)", flexWrap: "wrap" }}>
        {[
          { label: "Open Recipe", reason: "Real file picker deferred" },
          { label: "Compile Recipe", reason: "Compiler backend required" },
          { label: "Ask AI to Draft", reason: "AI workflow deferred; cannot bypass review" },
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

import { getRecipe, getRunSummary } from "../mock-data/helpers";

export default function RecipePage() {
  const recipe = getRecipe();
  const summary = getRunSummary();

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        Recipe
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
          { title: "Recipe name", value: recipe.id },
          { title: "Source", value: "static mock data" },
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
              { key: "Recipe hash", value: summary.recipeHash.slice(0, 8) + "..." + summary.recipeHash.slice(-6) },
              { key: "Resolved hash", value: "unknown" },
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

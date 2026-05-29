import { getRunManifest, getRawArtifactSummary, getIndexEntries } from "../mock-data/helpers";

export default function RawDataPreviewPage() {
  const manifest = getRunManifest();
  const raw = getRawArtifactSummary();
  const indexEntries = getIndexEntries();

  const artifacts = [
    { path: manifest.artifact_paths.manifest, type: "manifest" as const, size: "—", role: "Run manifest and artifact index", parsed: "Yes" },
    { path: manifest.artifact_paths.events, type: "events" as const, size: "—", role: "Run event log", parsed: "Yes" },
    { path: manifest.artifact_paths.index, type: "index" as const, size: "—", role: "Raw data index entries", parsed: "Yes" },
    { path: manifest.artifact_paths.raw_bin, type: "rawbin" as const, size: `${raw.sizeBytes.toLocaleString()} bytes`, role: "OE1022D raw binary frames", parsed: "No" },
    { path: manifest.artifact_paths.station_snapshot, type: "metadata" as const, size: "—", role: "Station state at run start", parsed: "Yes" },
    { path: manifest.artifact_paths.recipe_lock, type: "metadata" as const, size: "—", role: "Locked recipe copy", parsed: "Yes" },
    { path: manifest.artifact_paths.resolved_recipe_lock, type: "metadata" as const, size: "—", role: "Locked resolved recipe", parsed: "Yes" },
    { path: manifest.artifact_paths.dry_run_plan_lock, type: "metadata" as const, size: "—", role: "Locked dry-run plan", parsed: "Yes" },
    { path: manifest.artifact_paths.safety_report_lock, type: "metadata" as const, size: "—", role: "Locked safety report", parsed: "Yes" },
  ];

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        Raw Data Preview
      </h1>

      <div
        style={{
          background: "var(--color-warning-soft)",
          borderLeft: "4px solid var(--color-warning)",
          padding: "var(--space-3) var(--space-4)",
          marginBottom: "var(--space-6)",
          fontSize: "var(--font-size-sm)",
          color: "var(--color-text)",
        }}
      >
        {raw.note}
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
          { title: "Rawbin file", value: raw.filename },
          { title: "Rawbin size", value: `${raw.sizeBytes.toLocaleString()} bytes` },
          { title: "Index entries", value: String(indexEntries.length) },
          { title: "Metadata files", value: "5" },
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
            <tr style={{ background: "var(--color-surface-alt)" }}>
              {["path", "type", "size", "role", "parsed by GUI-M0"].map((h) => (
                <th
                  key={h}
                  style={{
                    padding: "10px 12px",
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
            {artifacts.map((art) => (
              <tr key={art.path} style={{ borderBottom: "1px solid var(--color-border)" }}>
                <td style={{ padding: "8px 12px", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)" }}>
                  {art.path}
                </td>
                <td style={{ padding: "8px 12px" }}>{art.type}</td>
                <td style={{ padding: "8px 12px" }}>{art.size}</td>
                <td style={{ padding: "8px 12px" }}>{art.role}</td>
                <td style={{ padding: "8px 12px" }}>
                  <span
                    style={{
                      fontSize: "var(--font-size-xs)",
                      padding: "2px 8px",
                      borderRadius: "var(--radius-sm)",
                      background:
                        art.parsed === "Yes"
                          ? "var(--color-success-soft)"
                          : "var(--color-danger-soft)",
                      color:
                        art.parsed === "Yes"
                          ? "var(--color-success)"
                          : "var(--color-danger)",
                    }}
                  >
                    {art.parsed}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

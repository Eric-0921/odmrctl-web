import { useMockMode } from "../context/MockModeContext";

export default function TopStatusBar() {
  const { mode } = useMockMode();

  const statusItems = [
    { label: "Project", value: "ODMR Automation", title: "Project name" },
    { label: "Phase", value: "M1 mock complete / M2 hardware bring-up pending", title: "Current development phase" },
    { label: "Mode", value: `GUI-M0 ${mode.toUpperCase()}`, title: "GUI operating mode" },
    { label: "Safety", value: "Allow", title: "Safety interlock decision" },
    { label: "Backend", value: "bundled static mock data", title: "Data source" },
    { label: "Run", value: "basic_odmr_mock_executor_run", title: "Current run identifier" },
  ];

  return (
    <header
      style={{
        height: 56,
        background: "var(--color-surface)",
        borderBottom: "1px solid var(--color-border)",
        display: "flex",
        alignItems: "center",
        padding: "0 var(--space-6)",
        gap: "var(--space-6)",
        flexShrink: 0,
        overflow: "hidden",
      }}
    >
      <span style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", flexShrink: 0 }}>
        ODMR Automation
      </span>

      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "var(--space-5)",
          overflow: "hidden",
          flex: 1,
          minWidth: 0,
        }}
      >
        {statusItems.slice(1).map((item) => (
          <span
            key={item.label}
            title={item.title}
            style={{
              fontSize: "var(--font-size-sm)",
              color:
                item.label === "Mode"
                  ? "var(--color-accent)"
                  : item.label === "Safety"
                  ? "var(--color-success)"
                  : "var(--color-text-muted)",
              fontWeight: item.label === "Mode" ? 600 : 400,
              background:
                item.label === "Mode"
                  ? "var(--color-accent-soft)"
                  : item.label === "Safety"
                  ? "var(--color-success-soft)"
                  : "transparent",
              padding:
                item.label === "Mode" || item.label === "Safety"
                  ? "2px 8px"
                  : "0",
              borderRadius: "var(--radius-sm)",
              flexShrink: 0,
              whiteSpace: "nowrap",
            }}
          >
            {item.label === "Mode" || item.label === "Safety" ? (
              <>
                {item.value}
              </>
            ) : (
              <>
                {item.label}: {item.value}
              </>
            )}
          </span>
        ))}
      </div>
    </header>
  );
}

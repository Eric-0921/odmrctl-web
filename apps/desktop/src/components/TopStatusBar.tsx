export default function TopStatusBar() {
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
      }}
    >
      <span style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>
        ODMR Automation
      </span>
      <span
        style={{
          fontSize: "var(--font-size-sm)",
          color: "var(--color-text-muted)",
        }}
      >
        Phase: M1 mock complete / M2 hardware bring-up pending
      </span>
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
        MOCK ONLY
      </span>
      <span
        style={{
          fontSize: "var(--font-size-sm)",
          color: "var(--color-text-muted)",
        }}
      >
        Safety: Allow
      </span>
      <span
        style={{
          fontSize: "var(--font-size-sm)",
          color: "var(--color-text-subtle)",
        }}
      >
        Backend: static mock data
      </span>
      <span
        style={{
          fontSize: "var(--font-size-sm)",
          color: "var(--color-text-subtle)",
          marginLeft: "auto",
        }}
      >
        Run: basic_odmr_mock_executor_run
      </span>
    </header>
  );
}

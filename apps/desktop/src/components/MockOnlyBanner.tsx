export default function MockOnlyBanner() {
  return (
    <div
      style={{
        background: "var(--color-primary-soft)",
        borderLeft: "4px solid var(--color-primary)",
        padding: "var(--space-3) var(--space-4)",
        fontSize: "var(--font-size-sm)",
        color: "var(--color-primary-strong)",
        fontWeight: 500,
        flexShrink: 0,
      }}
    >
      GUI-M0 MOCK VIEWER — No hardware access. No executor connection. Real
      controls disabled.
    </div>
  );
}

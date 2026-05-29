import { useMockMode } from "../context/MockModeContext";

export default function MockOnlyBanner() {
  const { mode, bannerCollapsed, toggleBanner } = useMockMode();

  if (mode !== "mock") return null;

  return (
    <div
      style={{
        background: "var(--color-primary-soft)",
        borderLeft: "4px solid var(--color-primary)",
        padding: bannerCollapsed
          ? "var(--space-1) var(--space-4)"
          : "var(--space-3) var(--space-4)",
        fontSize: "var(--font-size-sm)",
        color: "var(--color-primary-strong)",
        fontWeight: 500,
        flexShrink: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: "var(--space-3)",
        transition: "padding var(--transition-fast)",
      }}
    >
      {bannerCollapsed ? (
        <span style={{ fontSize: "var(--font-size-xs)", fontWeight: 600 }}>
          GUI-M0 MOCK VIEWER
        </span>
      ) : (
        <span>
          GUI-M0 MOCK VIEWER — No hardware access. No executor connection. Real
          controls disabled.
        </span>
      )}
      <button
        onClick={toggleBanner}
        title={bannerCollapsed ? "Expand banner" : "Collapse banner"}
        style={{
          background: "transparent",
          border: "1px solid var(--color-primary)",
          color: "var(--color-primary)",
          borderRadius: "var(--radius-sm)",
          padding: "2px 8px",
          fontSize: "var(--font-size-xs)",
          fontWeight: 600,
          cursor: "pointer",
          flexShrink: 0,
        }}
        aria-expanded={!bannerCollapsed}
        aria-label={bannerCollapsed ? "Expand mock mode banner" : "Collapse mock mode banner"}
      >
        {bannerCollapsed ? "▲" : "▼"}
      </button>
    </div>
  );
}

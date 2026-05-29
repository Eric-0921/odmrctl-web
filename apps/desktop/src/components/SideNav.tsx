import { NavLink } from "react-router-dom";

const navItems = [
  { path: "/", label: "Dashboard" },
  { path: "/devices", label: "Devices" },
  { path: "/recipe", label: "Recipe" },
  { path: "/dry-run", label: "Dry Run" },
  { path: "/safety", label: "Safety" },
  { path: "/events", label: "Events" },
  { path: "/raw-data", label: "Raw Data Preview" },
  { path: "/about", label: "About / Boundaries" },
];

const disabledItems = [
  { label: "Live Chart", reason: "requires backend trace stream" },
  { label: "Run Control", reason: "requires executor backend" },
  { label: "Magnetic Planner", reason: "future module" },
  { label: "Settings", reason: "future module" },
];

export default function SideNav() {
  return (
    <nav
      style={{
        width: 232,
        background: "var(--color-surface)",
        borderRight: "1px solid var(--color-border)",
        display: "flex",
        flexDirection: "column",
        padding: "var(--space-4) 0",
        flexShrink: 0,
        overflow: "auto",
      }}
    >
      {navItems.map((item) => (
        <NavLink
          key={item.path}
          to={item.path}
          style={({ isActive }) => ({
            display: "block",
            padding: "10px var(--space-4)",
            fontSize: "var(--font-size-sm)",
            color: isActive
              ? "var(--color-primary)"
              : "var(--color-text-muted)",
            textDecoration: "none",
            borderLeft: isActive
              ? "3px solid var(--color-primary)"
              : "3px solid transparent",
            fontWeight: isActive ? 600 : 400,
            background: isActive
              ? "var(--color-primary-soft)"
              : "transparent",
          })}
        >
          {item.label}
        </NavLink>
      ))}
      <div
        style={{
          marginTop: "var(--space-4)",
          paddingTop: "var(--space-4)",
          borderTop: "1px solid var(--color-border)",
        }}
      >
        {disabledItems.map((item) => (
          <div
            key={item.label}
            style={{
              padding: "10px var(--space-4)",
              fontSize: "var(--font-size-sm)",
              color: "var(--color-disabled-text)",
              display: "flex",
              alignItems: "center",
              gap: "var(--space-2)",
            }}
          >
            <span>{item.label}</span>
            <span
              style={{
                fontSize: "var(--font-size-xs)",
                padding: "1px 6px",
                borderRadius: "var(--radius-sm)",
                background: "var(--color-disabled-bg)",
              }}
            >
              {item.reason}
            </span>
          </div>
        ))}
      </div>
    </nav>
  );
}

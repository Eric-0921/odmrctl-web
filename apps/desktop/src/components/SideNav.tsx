import { NavLink } from "react-router-dom";

const navItems = [
  { path: "/", label: "设备工作台" },
  { path: "/experiment-plan", label: "实验计划" },
  { path: "/live-chart", label: "实时曲线" },
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
    </nav>
  );
}

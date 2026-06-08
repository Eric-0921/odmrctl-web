import { useMockMode } from "../context/MockModeContext";

export default function TopStatusBar() {
  const { mode } = useMockMode();

  const statusItems = [
    { label: "项目", value: "ODMR 自动化", title: "项目名称" },
    { label: "阶段", value: "设备工作台 / 实验计划草稿", title: "当前开发阶段" },
    { label: "模式", value: `GUI-M0 ${mode.toUpperCase()}`, title: "GUI 运行模式" },
    { label: "安全", value: "允许", title: "安全联锁判定" },
    { label: "后端", value: "Tauri typed commands / 本地静态数据", title: "数据来源" },
    { label: "运行", value: "basic_odmr_mock_executor_run", title: "当前运行标识" },
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
        ODMR 自动化
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
                item.label === "模式"
                  ? "var(--color-accent)"
                  : item.label === "安全"
                  ? "var(--color-success)"
                  : "var(--color-text-muted)",
              fontWeight: item.label === "模式" ? 600 : 400,
              background:
                item.label === "模式"
                  ? "var(--color-accent-soft)"
                  : item.label === "安全"
                  ? "var(--color-success-soft)"
                  : "transparent",
              padding:
                item.label === "模式" || item.label === "安全"
                  ? "2px 8px"
                  : "0",
              borderRadius: "var(--radius-sm)",
              flexShrink: 0,
              whiteSpace: "nowrap",
            }}
          >
            {item.label === "模式" || item.label === "安全" ? (
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

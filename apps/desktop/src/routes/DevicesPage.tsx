const devices = [
  {
    name: "SMB100A",
    role: "RF / microwave signal generator",
    required: true,
    connection: "unavailable in GUI-M0",
    lastKnownState: "static bundled data",
  },
  {
    name: "OE1022D",
    role: "DSP lock-in amplifier / acquisition source",
    required: true,
    connection: "unavailable in GUI-M0",
    lastKnownState: "static bundled data",
  },
  {
    name: "Laser Controller",
    role: "laser controller placeholder",
    required: false,
    connection: "unavailable in GUI-M0",
    lastKnownState: "static bundled data",
  },
  {
    name: "Magnetic Axes",
    role: "X/Y/Z magnetic field or current controller placeholder",
    required: false,
    connection: "unavailable in GUI-M0",
    lastKnownState: "static bundled data",
  },
];

const disabledControls: Record<string, { label: string; reason: string }[]> = {
  SMB100A: [
    { label: "Connect", reason: "M2 bring-up only" },
    { label: "Probe", reason: "Forbidden in GUI-M0" },
    { label: "Configure", reason: "Mock viewer only" },
    { label: "Output ON", reason: "Forbidden in GUI-M0" },
    { label: "MOD ON", reason: "Forbidden in GUI-M0" },
  ],
  OE1022D: [
    { label: "Connect", reason: "M2 bring-up only" },
    { label: "Probe", reason: "Forbidden in GUI-M0" },
    { label: "Configure", reason: "Mock viewer only" },
  ],
  "Laser Controller": [
    { label: "Connect", reason: "M2 bring-up only" },
    { label: "Emission ON", reason: "Forbidden in GUI-M0" },
  ],
  "Magnetic Axes": [
    { label: "Connect", reason: "M2 bring-up only" },
    { label: "Set B vector", reason: "Mock viewer only" },
    { label: "Ramp current", reason: "Forbidden in GUI-M0" },
    { label: "Safe zero", reason: "Forbidden in GUI-M0" },
  ],
};

export default function DevicesPage() {
  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        Devices
      </h1>
      <div
        style={{
          background: "var(--color-accent-soft)",
          borderLeft: "4px solid var(--color-accent)",
          padding: "var(--space-3) var(--space-4)",
          marginBottom: "var(--space-6)",
          fontSize: "var(--font-size-sm)",
          color: "var(--color-text)",
        }}
      >
        No serial / USB / VISA / TCP socket probing exists in GUI-M0.
      </div>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(2, 1fr)",
          gap: "var(--space-4)",
        }}
      >
        {devices.map((dev) => (
          <div
            key={dev.name}
            style={{
              background: "var(--color-surface)",
              borderRadius: "var(--radius-md)",
              padding: "var(--space-4)",
              boxShadow: "var(--shadow-card)",
              border: "1px solid var(--color-border)",
            }}
          >
            <h2 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-2)" }}>
              {dev.name}
            </h2>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-3)" }}>
              <div>Role: {dev.role}</div>
              <div>Required by recipe: {dev.required ? "yes" : "no"}</div>
              <div>Connection status: {dev.connection}</div>
              <div>Mock status: static snapshot only</div>
              <div>Last known state: {dev.lastKnownState}</div>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              {disabledControls[dev.name]?.map((ctrl) => (
                <div key={ctrl.label} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                  <button
                    disabled
                    style={{
                      padding: "4px 12px",
                      borderRadius: "var(--radius-sm)",
                      border: "1px solid var(--color-border)",
                      background: "var(--color-disabled-bg)",
                      color: "var(--color-disabled-text)",
                      cursor: "not-allowed",
                      fontSize: "var(--font-size-sm)",
                    }}
                  >
                    {ctrl.label}
                  </button>
                  <span style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-subtle)" }}>
                    {ctrl.reason}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

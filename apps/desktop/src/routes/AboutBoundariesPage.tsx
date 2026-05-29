export default function AboutBoundariesPage() {
  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        About / Boundaries
      </h1>

      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-6)",
          border: "1px solid var(--color-border)",
          marginBottom: "var(--space-6)",
        }}
      >
        <h2 style={{ fontSize: "var(--font-size-xl)", marginBottom: "var(--space-4)" }}>
          Boundary Statement
        </h2>
        <div
          style={{
            fontSize: "var(--font-size-md)",
            lineHeight: 1.6,
            color: "var(--color-text)",
          }}
        >
          <p style={{ marginBottom: "var(--space-3)" }}>
            This GUI is mock-only.
          </p>
          <p style={{ marginBottom: "var(--space-3)" }}>
            It does not connect to devices.
          </p>
          <p style={{ marginBottom: "var(--space-3)" }}>
            It does not call executor.
          </p>
          <p style={{ marginBottom: "var(--space-3)" }}>
            It does not send SCPI.
          </p>
          <p style={{ marginBottom: "var(--space-3)" }}>
            It does not read OE1022D RALL?.
          </p>
          <p style={{ marginBottom: "var(--space-3)" }}>
            It does not write experiment data.
          </p>
          <p>
            Future M2 integration must go through backend APIs, executor, and
            safety interlock.
          </p>
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-4)", marginBottom: "var(--space-6)" }}>
        <div
          style={{
            background: "var(--color-surface)",
            borderRadius: "var(--radius-md)",
            padding: "var(--space-4)",
            border: "1px solid var(--color-border)",
          }}
        >
          <h3 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-3)", color: "var(--color-success)" }}>
            Allowed in M0
          </h3>
          <ul style={{ fontSize: "var(--font-size-sm)", lineHeight: 1.8, paddingLeft: "var(--space-5)", color: "var(--color-text)" }}>
            <li>Display mock run summary</li>
            <li>Display dry-run plan</li>
            <li>Display safety report</li>
            <li>Display events</li>
            <li>Display artifact inventory</li>
            <li>Display disabled future controls</li>
          </ul>
        </div>

        <div
          style={{
            background: "var(--color-surface)",
            borderRadius: "var(--radius-md)",
            padding: "var(--space-4)",
            border: "1px solid var(--color-border)",
          }}
        >
          <h3 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-3)", color: "var(--color-danger)" }}>
            Forbidden in M0
          </h3>
          <ul style={{ fontSize: "var(--font-size-sm)", lineHeight: 1.8, paddingLeft: "var(--space-5)", color: "var(--color-text)" }}>
            <li>serial / USB / VISA / TCP socket access</li>
            <li>SCPI sending</li>
            <li>executor calls</li>
            <li>hardware polling</li>
            <li>raw data parsing</li>
            <li>run data writing</li>
            <li>AI live hardware control</li>
          </ul>
        </div>
      </div>

      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          border: "1px solid var(--color-border)",
        }}
      >
        <h3 style={{ fontSize: "var(--font-size-lg)", marginBottom: "var(--space-3)" }}>
          Future M1 / M2 Integration Path
        </h3>
        <div style={{ fontSize: "var(--font-size-sm)", lineHeight: 1.6, color: "var(--color-text-muted)" }}>
          <p style={{ marginBottom: "var(--space-2)" }}>
            <strong>M1</strong> may add read-only backend APIs: mock run
            summary listing, safe static file loading, mock replay timeline,
            downsampled chart preview.
          </p>
          <p>
            <strong>M2</strong> may add real backend commands: connect_device,
            get_device_status_snapshot, request_run_start,
            request_safe_shutdown. Even in M2, GUI sends user intent; executor
            owns run authority; safety owns allow/reject.
          </p>
        </div>
      </div>
    </div>
  );
}

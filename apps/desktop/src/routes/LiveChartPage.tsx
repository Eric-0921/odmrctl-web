import { useState, useEffect, useRef, useCallback } from "react";
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid,
  Tooltip, Legend, ResponsiveContainer,
} from "recharts";
import type { TraceSnapshot } from "../types/liveTrace";

type ConnState =
  | { tag: "disconnected" }
  | { tag: "connecting" }
  | { tag: "connected"; snapshot: TraceSnapshot }
  | { tag: "error"; message: string };

const SERVER_URL = "http://127.0.0.1:9876";

const card: React.CSSProperties = {
  background: "var(--color-surface)",
  border: "1px solid var(--color-border)",
  borderRadius: "var(--radius-md)",
  padding: "var(--space-3)",
};

const statLabel: React.CSSProperties = {
  fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)"
};
const statValue: React.CSSProperties = {
  fontSize: "var(--font-size-lg)", fontWeight: 700
};

const sectionTitle: React.CSSProperties = {
  fontSize: "var(--font-size-lg)", fontWeight: 600,
  marginBottom: "var(--space-3)", color: "var(--color-text)",
};

export default function LiveChartPage() {
  const [conn, setConn] = useState<ConnState>({ tag: "disconnected" });
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const poll = useCallback(async () => {
    try {
      const resp = await fetch(`${SERVER_URL}/api/trace`);
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const snap: TraceSnapshot = await resp.json();
      setConn({ tag: "connected", snapshot: snap });
    } catch (e) {
      setConn({ tag: "error", message: String(e) });
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    }
  }, []);

  const connect = useCallback(() => {
    setConn({ tag: "connecting" });
    poll(); // immediate first fetch
    intervalRef.current = setInterval(poll, 50); // 20 Hz
  }, [poll]);

  useEffect(() => {
    connect();
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [connect]);

  // Disconnected state
  if (conn.tag === "disconnected") {
    return (
      <div style={{ padding: "var(--space-6)" }}>
        <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700, marginBottom: "var(--space-4)" }}>
          Live Chart
        </h1>
        <div style={{ ...card, textAlign: "center", padding: "var(--space-8)", maxWidth: 560, margin: "0 auto" }}>
          <p style={{ color: "var(--color-text-muted)", marginBottom: "var(--space-4)" }}>
            Start the OE1022D live trace server first:
          </p>
          <pre style={{
            background: "var(--color-disabled-bg)", padding: "var(--space-3)",
            borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-sm)",
            textAlign: "left", marginBottom: "var(--space-4)",
          }}>
            cargo run -p odmr-live-server -- --port /dev/cu.usbmodem395D388533371
          </pre>
          <button onClick={connect} style={{
            padding: "10px 24px", fontSize: "var(--font-size-base)", fontWeight: 600,
            background: "var(--color-primary)", color: "#fff", border: "none",
            borderRadius: "var(--radius-md)", cursor: "pointer",
          }}>
            Connect
          </button>
        </div>
      </div>
    );
  }

  // Connecting state
  if (conn.tag === "connecting") {
    return (
      <div style={{ padding: "var(--space-6)" }}>
        <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700 }}>Live Chart</h1>
        <p style={{ color: "var(--color-text-muted)", marginTop: "var(--space-4)" }}>
          Connecting to {SERVER_URL}...
        </p>
      </div>
    );
  }

  // Error state
  if (conn.tag === "error") {
    return (
      <div style={{ padding: "var(--space-6)" }}>
        <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700 }}>Live Chart</h1>
        <div style={{ ...card, borderColor: "var(--color-danger)", background: "var(--color-danger-soft)", marginTop: "var(--space-4)", maxWidth: 640 }}>
          <p style={{ fontWeight: 600, color: "var(--color-danger)", marginBottom: "var(--space-2)" }}>
            Connection Error
          </p>
          <pre style={{ fontSize: "var(--font-size-sm)", whiteSpace: "pre-wrap" }}>{conn.message}</pre>
          <button onClick={() => connect()} style={{
            marginTop: "var(--space-3)", padding: "6px 16px",
            background: "var(--color-primary)", color: "#fff", border: "none",
            borderRadius: "var(--radius-sm)", cursor: "pointer", fontWeight: 600,
          }}>
            Retry
          </button>
        </div>
      </div>
    );
  }

  // Connected state — render chart
  const { snapshot } = conn;
  const dupPct = (snapshot.dup_rate * 100).toFixed(1);
  const avgRead = (snapshot.avg_read_us / 1000).toFixed(1);

  // Chart data: last 2000 points, sliding window
  const chartData = snapshot.points.slice(-2000).map((p) => ({
    t: p.elapsed_s,
    bx: p.bx_mv,
    by: p.by_mv,
  }));

  // X-axis domain: last 2 seconds
  const lastT = chartData.length > 0 ? chartData[chartData.length - 1].t : 0;
  const xMin = Math.max(0, lastT - 2);
  const xMax = lastT;

  return (
    <div style={{ padding: "var(--space-6)", paddingBottom: "var(--space-2)" }}>
      <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700, marginBottom: "var(--space-1)" }}>
        Live Chart
      </h1>
      <p style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-sm)", marginBottom: "var(--space-4)" }}>
        OE1022D B-channel real-time trace · {SERVER_URL}
      </p>

      {/* Status bar */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "var(--space-3)", marginBottom: "var(--space-4)" }}>
        <div style={card}>
          <div style={statLabel}>Frames (total / unique)</div>
          <div style={statValue}>{snapshot.frames_total} / {snapshot.frames_unique}</div>
        </div>
        <div style={card}>
          <div style={statLabel}>Duplicate Rate</div>
          <div style={statValue}>{dupPct}%</div>
        </div>
        <div style={card}>
          <div style={statLabel}>Avg Read Time</div>
          <div style={statValue}>{avgRead} ms</div>
        </div>
        <div style={card}>
          <div style={statLabel}>Points in Buffer</div>
          <div style={statValue}>{snapshot.points.length}</div>
        </div>
      </div>

      {/* B-X, B-Y vs Time */}
      <h2 style={sectionTitle}>B-Channel Signal vs Time</h2>
      <div style={{ ...card, marginBottom: "var(--space-4)", height: 360 }}>
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData} margin={{ top: 16, right: 24, left: 16, bottom: 16 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
            <XAxis
              dataKey="t"
              type="number"
              domain={[xMin, xMax]}
              label={{ value: "Time (s)", position: "insideBottom", offset: -8, style: { fill: "var(--color-text-muted)", fontSize: 12 } }}
              tick={{ fontSize: 11, fill: "var(--color-text-muted)" }}
              tickFormatter={(v: number) => v.toFixed(1)}
            />
            <YAxis
              label={{ value: "Signal (mV)", angle: -90, position: "insideLeft", offset: 8, style: { fill: "var(--color-text-muted)", fontSize: 12 } }}
              tick={{ fontSize: 11, fill: "var(--color-text-muted)" }}
            />
            <Tooltip
              contentStyle={{
                background: "var(--color-surface)",
                border: "1px solid var(--color-border)",
                borderRadius: "var(--radius-sm)",
                fontSize: "var(--font-size-sm)",
              }}
              formatter={(value) => [Number(value).toExponential(4)]}
              labelFormatter={(label) => `t = ${Number(label).toFixed(3)} s`}
            />
            <Legend wrapperStyle={{ fontSize: "var(--font-size-sm)" }} />
            <Line
              type="monotone" dataKey="bx" name="B-X" stroke="var(--color-primary)"
              dot={false} strokeWidth={1.5} isAnimationActive={false}
            />
            <Line
              type="monotone" dataKey="by" name="B-Y" stroke="var(--color-accent)"
              dot={false} strokeWidth={1.5} isAnimationActive={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>

      <p style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)", marginBottom: "var(--space-4)" }}>
        {chartData.length} points displayed · 2 s sliding window · B-X (in-phase) / B-Y (quadrature) · 1 kHz equivalent resolution
      </p>

      {/* Boundary banner */}
      <div style={{
        background: "var(--color-primary-soft)", borderLeft: "4px solid var(--color-primary)",
        padding: "var(--space-3) var(--space-4)", fontSize: "var(--font-size-sm)",
        color: "var(--color-primary-strong)", fontWeight: 500, borderRadius: "var(--radius-sm)",
        marginTop: "var(--space-4)",
      }}>
        LIVE TRACE — Read-only chart display. No recipe execution. Data served by odmr-live-server sidecar over localhost HTTP.
      </div>
    </div>
  );
}

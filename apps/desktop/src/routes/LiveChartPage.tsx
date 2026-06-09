import { useCallback, useEffect, useRef, useState } from "react";
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { TracePoint, TraceSnapshot } from "../types/liveTrace";

type ConnState =
  | { tag: "idle" }
  | { tag: "connecting" }
  | { tag: "connected"; snapshot: TraceSnapshot }
  | { tag: "retrying"; message: string; snapshot?: TraceSnapshot }
  | { tag: "stopped"; snapshot?: TraceSnapshot };

const SERVER_URL = "http://127.0.0.1:9876";
const POLL_MS = 250;
const DISPLAY_POINTS = 1000;

const card: React.CSSProperties = {
  background: "var(--color-surface)",
  border: "1px solid var(--color-border)",
  borderRadius: "var(--radius-md)",
  padding: "var(--space-3)",
};

const btnPrimary: React.CSSProperties = {
  padding: "8px 14px",
  borderRadius: "var(--radius-sm)",
  border: "none",
  background: "var(--color-primary)",
  color: "#fff",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  cursor: "pointer",
};

const btnSecondary: React.CSSProperties = {
  padding: "8px 14px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--color-border)",
  background: "var(--color-surface)",
  color: "var(--color-text)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  cursor: "pointer",
};

const badge = (kind: "ok" | "warn" | "off"): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-sm)",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  background:
    kind === "ok"
      ? "var(--color-success-soft)"
      : kind === "warn"
        ? "var(--color-warning-soft)"
        : "var(--color-disabled-bg)",
  color:
    kind === "ok"
      ? "var(--color-success)"
      : kind === "warn"
        ? "var(--color-warning)"
        : "var(--color-disabled-text)",
});

export default function LiveChartPage() {
  const [conn, setConn] = useState<ConnState>({ tag: "idle" });
  const [displayRunning, setDisplayRunning] = useState(false);
  const [saving, setSaving] = useState(false);
  const [savedPoints, setSavedPoints] = useState(0);
  const [source, setSource] = useState<"xy" | "x" | "y">("xy");
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const saveBufferRef = useRef<TracePoint[]>([]);

  const stopTimer = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  const poll = useCallback(async () => {
    try {
      const resp = await fetch(`${SERVER_URL}/api/trace`, { cache: "no-store" });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const snap: TraceSnapshot = await resp.json();
      const capped = { ...snap, points: snap.points.slice(-DISPLAY_POINTS) };
      if (saving) {
        saveBufferRef.current.push(...capped.points);
        setSavedPoints(saveBufferRef.current.length);
      }
      setConn({ tag: "connected", snapshot: capped });
    } catch (e) {
      setConn((prev) => ({
        tag: "retrying",
        message: String(e),
        snapshot: "snapshot" in prev ? prev.snapshot : undefined,
      }));
    }
  }, [saving]);

  const startDisplay = useCallback(() => {
    stopTimer();
    setDisplayRunning(true);
    setConn({ tag: "connecting" });
    void poll();
    intervalRef.current = setInterval(() => void poll(), POLL_MS);
  }, [poll, stopTimer]);

  const stopDisplay = useCallback(() => {
    stopTimer();
    setDisplayRunning(false);
    setConn((prev) => ({ tag: "stopped", snapshot: "snapshot" in prev ? prev.snapshot : undefined }));
  }, [stopTimer]);

  useEffect(() => stopTimer, [stopTimer]);

  const snapshot = "snapshot" in conn ? conn.snapshot : undefined;
  const chartData = (snapshot?.points ?? []).map((p) => ({
    t: p.elapsed_s,
    bx: p.bx_mv,
    by: p.by_mv,
  }));
  const lastT = chartData.length > 0 ? chartData[chartData.length - 1].t : 0;
  const xMin = Math.max(0, lastT - 5);
  const statusKind = conn.tag === "connected" ? "ok" : conn.tag === "retrying" ? "warn" : "off";

  return (
    <div style={{ padding: "var(--space-6)", paddingBottom: "var(--space-2)" }}>
      <h1 style={{ fontSize: "var(--font-size-2xl)", fontWeight: 700, marginBottom: "var(--space-1)" }}>
        实时曲线
      </h1>
      <p style={{ color: "var(--color-text-muted)", fontSize: "var(--font-size-sm)", marginBottom: "var(--space-4)" }}>
        OE1022D 实时显示。显示链路与真实采集应共享后端数据源，避免两个进程同时抢占 OE 串口。
      </p>

      <div style={{ ...card, marginBottom: "var(--space-4)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: "var(--space-3)", flexWrap: "wrap" }}>
          <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap", alignItems: "center" }}>
            <button onClick={startDisplay} disabled={displayRunning} style={displayRunning ? btnSecondary : btnPrimary}>开始实时显示</button>
            <button onClick={stopDisplay} disabled={!displayRunning} style={btnSecondary}>停止实时显示</button>
            <button
              onClick={() => {
                saveBufferRef.current = [];
                setSavedPoints(0);
                setSaving(true);
              }}
              disabled={saving}
              style={btnSecondary}
            >
              开始保存
            </button>
            <button onClick={() => setSaving(false)} disabled={!saving} style={btnSecondary}>停止保存</button>
            <button
              onClick={() => {
                saveBufferRef.current = [];
                setSavedPoints(0);
                setConn({ tag: "idle" });
              }}
              style={btnSecondary}
            >
              清空曲线
            </button>
          </div>
          <label style={{ display: "grid", gap: 4, fontSize: "var(--font-size-xs)" }}>
            数据源
            <select value={source} onChange={(event) => setSource(event.target.value as typeof source)} style={{ padding: "6px 8px" }}>
              <option value="xy">Ch-B X/Y</option>
              <option value="x">Ch-B X</option>
              <option value="y">Ch-B Y</option>
            </select>
          </label>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)", flexWrap: "wrap", marginTop: "var(--space-3)", fontSize: "var(--font-size-xs)" }}>
          <span style={badge(statusKind)}>状态：{conn.tag === "retrying" ? "连接中断，自动重试" : conn.tag}</span>
          <span style={badge(saving ? "ok" : "off")}>保存：{saving ? "进行中" : "停止"}</span>
          <span>保存点数：{savedPoints}</span>
          <span>显示点数：{chartData.length}</span>
          <span>轮询间隔：{POLL_MS} ms</span>
        </div>
        {conn.tag === "retrying" && (
          <div style={{ marginTop: "var(--space-2)", color: "var(--color-warning)", fontSize: "var(--font-size-xs)" }}>
            {conn.message}。页面不会白屏；请确认后端实时数据源是否启动，或等待执行器共享数据源接入。
          </div>
        )}
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "var(--space-3)", marginBottom: "var(--space-4)" }}>
        <Stat label="Frames total / unique" value={`${snapshot?.frames_total ?? 0} / ${snapshot?.frames_unique ?? 0}`} />
        <Stat label="Duplicate rate" value={`${(((snapshot?.dup_rate ?? 0) * 100)).toFixed(1)}%`} />
        <Stat label="Avg read time" value={`${((snapshot?.avg_read_us ?? 0) / 1000).toFixed(1)} ms`} />
        <Stat label="Buffer points" value={`${snapshot?.points.length ?? 0}`} />
      </div>

      <div style={{ ...card, marginBottom: "var(--space-4)", height: 420 }}>
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData} margin={{ top: 16, right: 24, left: 16, bottom: 16 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
            <XAxis
              dataKey="t"
              type="number"
              domain={[xMin, lastT || 5]}
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
            {(source === "xy" || source === "x") && (
              <Line type="monotone" dataKey="bx" name="Ch-B X" stroke="var(--color-primary)" dot={false} strokeWidth={1.5} isAnimationActive={false} />
            )}
            {(source === "xy" || source === "y") && (
              <Line type="monotone" dataKey="by" name="Ch-B Y" stroke="var(--color-accent)" dot={false} strokeWidth={1.5} isAnimationActive={false} />
            )}
          </LineChart>
        </ResponsiveContainer>
      </div>

      <div style={{
        background: "var(--color-primary-soft)",
        borderLeft: "4px solid var(--color-primary)",
        padding: "var(--space-3) var(--space-4)",
        fontSize: "var(--font-size-sm)",
        color: "var(--color-primary-strong)",
        fontWeight: 500,
        borderRadius: "var(--radius-sm)",
      }}>
        当前版本显示端仍读取 localhost 实时源；真实实验运行时必须切换到 executor 共享数据源，不能另开 OE1022D 串口采集进程。
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div style={card}>
      <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-muted)" }}>{label}</div>
      <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 700 }}>{value}</div>
    </div>
  );
}

import { useState, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  RecipeValidationResult,
  ResolvedPreview,
  DryRunPreview,
  SafetyPreview,
  CommandPlanPreview,
  SafetyFinding,
} from "../types/recipe";
import { validateRecipe } from "../utils/recipeValidation";
import {
  buildResolvedPreview,
  buildDryRunPreview,
  buildSafetyPreview,
  buildCommandPlanPreview,
} from "../utils/recipePreview";

const EXAMPLE_RECIPE_JSON = JSON.stringify(
  {
    schema_version: "0.2.0",
    kind: "two_device_odmr_like_sweep_recipe",
    id: "m3_4_two_device_sweep",
    description:
      "Recipe-shaped SMB100A/OE1022D software-stepped run; no physical ODMR response required.",
    devices: {
      smb100a: { device_id: "smb100a_main", mode: "real_or_fake_by_runtime" },
      oe1022d: { device_id: "oe1022d_main", mode: "real_or_fake_by_runtime" },
      magnetic: { in_scope: false },
    },
    rf: {
      start_hz: 2878000000,
      stop_hz: 2886000000,
      points: 11,
      power_dbm: -30,
      max_power_dbm: -20,
    },
    modulation: {
      fm_source: "INT",
      fm_deviation_hz: 4000000,
      max_fm_deviation_hz: 5000000,
      internal_lf: {
        enabled: true,
        frequency_hz: 500,
        shape: "SQU",
        voltage_v: 0.137,
        lf_output_enabled: false,
      },
    },
    acquisition: {
      frames_per_step: 5,
      repeat_count: 2,
      inter_frame_delay_ms: 20,
    },
    safety: {
      require_operator_approval: true,
      no_internal_sweep: true,
      no_csv: true,
      no_gui: true,
      no_magnetic: true,
      physical_response_required: false,
    },
  },
  null,
  2
);

// ---------------------------------------------------------------------------
// Small UI helpers
// ---------------------------------------------------------------------------

function Badge({
  children,
  variant,
}: {
  children: React.ReactNode;
  variant: "success" | "warning" | "danger" | "info";
}) {
  const colors = {
    success: {
      bg: "var(--color-success-soft)",
      text: "var(--color-success)",
    },
    warning: {
      bg: "var(--color-warning-soft)",
      text: "var(--color-warning)",
    },
    danger: { bg: "var(--color-danger-soft)", text: "var(--color-danger)" },
    info: { bg: "var(--color-primary-soft)", text: "var(--color-primary)" },
  };
  const c = colors[variant];
  return (
    <span
      style={{
        display: "inline-block",
        fontSize: "var(--font-size-xs)",
        fontWeight: 600,
        padding: "2px 8px",
        borderRadius: "var(--radius-sm)",
        background: c.bg,
        color: c.text,
      }}
    >
      {children}
    </span>
  );
}

function Card({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        background: "var(--color-surface)",
        borderRadius: "var(--radius-md)",
        padding: "var(--space-4)",
        border: "1px solid var(--color-border)",
        boxShadow: "var(--shadow-card)",
      }}
    >
      {children}
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h2
      style={{
        fontSize: "var(--font-size-lg)",
        fontWeight: 600,
        marginBottom: "var(--space-3)",
        marginTop: 0,
      }}
    >
      {children}
    </h2>
  );
}

function Panel({
  children,
  title,
}: {
  children: React.ReactNode;
  title: string;
}) {
  return (
    <div style={{ marginBottom: "var(--space-6)" }}>
      <SectionTitle>{title}</SectionTitle>
      {children}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page component
// ---------------------------------------------------------------------------

export default function RecipeViewerPage() {
  const [recipeText, setRecipeText] = useState(EXAMPLE_RECIPE_JSON);
  const [parseError, setParseError] = useState<string | null>(null);

  const validation = useMemo<RecipeValidationResult>(() => {
    setParseError(null);
    return validateRecipe(recipeText);
  }, [recipeText]);

  const previews = useMemo(() => {
    if (!validation.recipe) return null;
    const recipe = validation.recipe;
    return {
      resolved: buildResolvedPreview(recipe),
      dryRun: buildDryRunPreview(recipe),
      safety: buildSafetyPreview(recipe),
      commandPlan: buildCommandPlanPreview(recipe),
    };
  }, [validation]);

  const handlePickFile = useCallback(async () => {
    try {
      const path = (await invoke("pick_recipe_file")) as string | null;
      if (path) {
        const text = (await invoke("read_recipe_file", { path })) as string;
        setRecipeText(text);
      }
    } catch (e) {
      setParseError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  const handleLoadExample = useCallback(() => {
    setRecipeText(EXAMPLE_RECIPE_JSON);
  }, []);

  const handleReset = useCallback(() => {
    setRecipeText("");
  }, []);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div>
      <h1 style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-4)" }}>
        M4.1 Recipe / Dry-run Viewer
      </h1>

      {/* 1. Boundary Banner */}
      <div
        style={{
          background: "var(--color-primary-soft)",
          borderLeft: "4px solid var(--color-primary)",
          padding: "var(--space-3) var(--space-4)",
          marginBottom: "var(--space-6)",
          fontSize: "var(--font-size-sm)",
          color: "var(--color-primary-strong)",
          fontWeight: 500,
        }}
      >
        Dry-run viewer only. No hardware connection. No recipe execution.
      </div>

      {/* 2. Recipe Input */}
      <Panel title="Recipe Input">
        <div style={{ display: "flex", gap: "var(--space-3)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
          <button
            onClick={handlePickFile}
            style={{
              padding: "6px 14px",
              borderRadius: "var(--radius-sm)",
              border: "1px solid var(--color-border)",
              background: "var(--color-surface-alt)",
              color: "var(--color-text)",
              cursor: "pointer",
              fontSize: "var(--font-size-sm)",
            }}
          >
            Load from File
          </button>
          <button
            onClick={handleLoadExample}
            style={{
              padding: "6px 14px",
              borderRadius: "var(--radius-sm)",
              border: "1px solid var(--color-border)",
              background: "var(--color-surface-alt)",
              color: "var(--color-text)",
              cursor: "pointer",
              fontSize: "var(--font-size-sm)",
            }}
          >
            Reset to Example
          </button>
          <button
            onClick={handleReset}
            style={{
              padding: "6px 14px",
              borderRadius: "var(--radius-sm)",
              border: "1px solid var(--color-border)",
              background: "var(--color-surface-alt)",
              color: "var(--color-text)",
              cursor: "pointer",
              fontSize: "var(--font-size-sm)",
            }}
          >
            Clear
          </button>
        </div>
        <textarea
          value={recipeText}
          onChange={(e) => setRecipeText(e.target.value)}
          spellCheck={false}
          style={{
            width: "100%",
            minHeight: 280,
            fontFamily: "var(--font-mono)",
            fontSize: "var(--font-size-xs)",
            lineHeight: 1.5,
            padding: "var(--space-3)",
            borderRadius: "var(--radius-md)",
            border: "1px solid var(--color-border)",
            background: "var(--color-bg)",
            color: "var(--color-text)",
            resize: "vertical",
            boxSizing: "border-box",
          }}
        />
        {parseError && (
          <div
            style={{
              marginTop: "var(--space-2)",
              color: "var(--color-danger)",
              fontSize: "var(--font-size-sm)",
            }}
          >
            File error: {parseError}
          </div>
        )}
      </Panel>

      {/* 3. Parse & Validation Status */}
      <Panel title="Validation">
        <div style={{ display: "flex", gap: "var(--space-3)", marginBottom: "var(--space-4)", flexWrap: "wrap" }}>
          <Card>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
              JSON Parse
            </div>
            <div>
              {validation.parseOk ? (
                <Badge variant="success">OK</Badge>
              ) : (
                <Badge variant="danger">FAILED</Badge>
              )}
            </div>
          </Card>
          <Card>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
              Shape Check
            </div>
            <div>
              {validation.shapeOk ? (
                <Badge variant="success">OK</Badge>
              ) : (
                <Badge variant="danger">FAILED</Badge>
              )}
            </div>
          </Card>
          <Card>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
              Value Check
            </div>
            <div>
              {validation.valueOk ? (
                validation.warnings.length > 0 ? (
                  <Badge variant="warning">OK WITH WARNINGS</Badge>
                ) : (
                  <Badge variant="success">OK</Badge>
                )
              ) : (
                <Badge variant="danger">FAILED</Badge>
              )}
            </div>
          </Card>
          <Card>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
              Overall
            </div>
            <div>
              {validation.parseOk && validation.shapeOk && validation.valueOk ? (
                validation.warnings.length > 0 ? (
                  <Badge variant="warning">ALLOW WITH WARNINGS</Badge>
                ) : (
                  <Badge variant="success">ALLOW</Badge>
                )
              ) : (
                <Badge variant="danger">REJECT</Badge>
              )}
            </div>
          </Card>
        </div>

        {/* Errors */}
        {(validation.shapeErrors.length > 0 || validation.valueErrors.length > 0) && (
          <div
            style={{
              background: "var(--color-danger-soft)",
              borderRadius: "var(--radius-md)",
              padding: "var(--space-3)",
              marginBottom: "var(--space-3)",
            }}
          >
            <div style={{ fontWeight: 600, color: "var(--color-danger)", fontSize: "var(--font-size-sm)", marginBottom: "var(--space-2)" }}>
              Errors
            </div>
            <ul style={{ margin: 0, paddingLeft: "var(--space-5)", fontSize: "var(--font-size-sm)", color: "var(--color-danger)" }}>
              {[...validation.shapeErrors, ...validation.valueErrors].map((e, i) => (
                <li key={i}>{e}</li>
              ))}
            </ul>
          </div>
        )}

        {/* Warnings */}
        {validation.warnings.length > 0 && (
          <div
            style={{
              background: "var(--color-warning-soft)",
              borderRadius: "var(--radius-md)",
              padding: "var(--space-3)",
            }}
          >
            <div style={{ fontWeight: 600, color: "var(--color-warning)", fontSize: "var(--font-size-sm)", marginBottom: "var(--space-2)" }}>
              Warnings
            </div>
            <ul style={{ margin: 0, paddingLeft: "var(--space-5)", fontSize: "var(--font-size-sm)", color: "var(--color-warning)" }}>
              {validation.warnings.map((w, i) => (
                <li key={i}>{w}</li>
              ))}
            </ul>
          </div>
        )}
      </Panel>

      {/* System Scan Recipe placeholder */}
      {validation.kind === "system_scan_recipe" && validation.parseOk && validation.shapeOk && (
        <Panel title="System Scan Recipe">
          <div
            style={{
              background: "var(--color-info-soft)",
              borderRadius: "var(--radius-md)",
              padding: "var(--space-4)",
              textAlign: "center",
            }}
          >
            <div style={{ fontWeight: 600, color: "var(--color-info)", marginBottom: "var(--space-2)" }}>
              system_scan_recipe recognized
            </div>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)" }}>
              Full GUI preview for system-level scan recipes is not yet implemented.
              <br />
              Use the Rust CLI compiler to expand and preview this recipe.
            </div>
          </div>
        </Panel>
      )}

      {/* Only show preview panels if recipe is valid */}
      {validation.recipe && previews && (
        <>
          {/* 4. Recipe Summary */}
          <Panel title="Recipe Summary">
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(4, 1fr)",
                gap: "var(--space-4)",
              }}
            >
              {[
                { label: "Recipe ID", value: validation.recipe.id },
                { label: "Kind", value: validation.recipe.kind },
                { label: "RF Start", value: `${(validation.recipe.rf.start_hz / 1e9).toFixed(3)} GHz` },
                { label: "RF Stop", value: `${(validation.recipe.rf.stop_hz / 1e9).toFixed(3)} GHz` },
                { label: "RF Points", value: String(validation.recipe.rf.points) },
                { label: "RF Power", value: `${validation.recipe.rf.power_dbm} dBm` },
                { label: "FM Source", value: validation.recipe.modulation.fm_source ?? "INT" },
                { label: "FM Deviation", value: `${(validation.recipe.modulation.fm_deviation_hz / 1e6).toFixed(1)} MHz` },
                { label: "Frames / Step", value: String(validation.recipe.acquisition.frames_per_step) },
                { label: "Repeat Count", value: String(validation.recipe.acquisition.repeat_count ?? 2) },
                { label: "Physical Response", value: (validation.recipe.safety.physical_response_required ?? false) ? "Required" : "Not required" },
                { label: "Magnetic", value: validation.recipe.devices.magnetic.in_scope ? "IN SCOPE" : "Not in scope" },
              ].map((item) => (
                <Card key={item.label}>
                  <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
                    {item.label}
                  </div>
                  <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, fontFamily: "var(--font-mono)" }}>
                    {item.value}
                  </div>
                </Card>
              ))}
            </div>
          </Panel>

          {/* 5. Resolved Recipe Preview */}
          <ResolvedPreviewPanel preview={previews.resolved} />

          {/* 6. Dry-run Plan Preview */}
          <DryRunPreviewPanel preview={previews.dryRun} />

          {/* 7. Safety Report Preview */}
          <SafetyPreviewPanel preview={previews.safety} />

          {/* 8. Command Plan Preview */}
          <CommandPlanPreviewPanel preview={previews.commandPlan} />
        </>
      )}
    </div>
  );
}


// ---------------------------------------------------------------------------
// Sub-panels
// ---------------------------------------------------------------------------

function ResolvedPreviewPanel({ preview }: { preview: ResolvedPreview }) {
  const freqPreview = preview.frequencies.slice(0, 5);
  const hasMore = preview.frequencies.length > 5;

  return (
    <Panel title="Resolved Recipe Preview">
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gap: "var(--space-4)",
          marginBottom: "var(--space-4)",
        }}
      >
        {[
          { label: "Step Count", value: String(preview.step_count) },
          { label: "Total Frames", value: String(preview.total_frames) },
          { label: "Est. Duration", value: `${preview.estimated_duration_s.toFixed(1)} s` },
          { label: "Devices", value: preview.device_list.join(", ") },
        ].map((item) => (
          <Card key={item.label}>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
              {item.label}
            </div>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, fontFamily: "var(--font-mono)" }}>
              {item.value}
            </div>
          </Card>
        ))}
      </div>
      <Card>
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-2)" }}>
          Frequency Grid ({preview.frequencies.length} frequencies)
        </div>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: "var(--font-size-sm)",
            display: "flex",
            flexWrap: "wrap",
            gap: "var(--space-2)",
          }}
        >
          {freqPreview.map((f, i) => (
            <span
              key={i}
              style={{
                background: "var(--color-surface-alt)",
                padding: "2px 8px",
                borderRadius: "var(--radius-sm)",
              }}
            >
              {(f / 1e9).toFixed(3)} GHz
            </span>
          ))}
          {hasMore && (
            <span style={{ color: "var(--color-text-muted)", padding: "2px 4px" }}>
              +{preview.frequencies.length - 5} more
            </span>
          )}
        </div>
      </Card>
    </Panel>
  );
}

function DryRunPreviewPanel({ preview }: { preview: DryRunPreview }) {
  return (
    <Panel title="Dry-run Plan Preview">
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gap: "var(--space-4)",
          marginBottom: "var(--space-4)",
        }}
      >
        {[
          { label: "Planned Steps", value: String(preview.step_count) },
          { label: "Total Frames", value: String(preview.total_frames) },
          { label: "SMB Set Commands", value: String(preview.smb_set_count) },
          { label: "SMB Query Commands", value: String(preview.smb_query_count) },
          { label: "OE Frames", value: String(preview.oe_frame_count) },
          { label: "Est. Duration", value: `${preview.estimated_duration_s.toFixed(1)} s` },
          { label: "Repeat Count", value: String(preview.repeat_count) },
          { label: "RF Points", value: String(preview.rf_points) },
        ].map((item) => (
          <Card key={item.label}>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
              {item.label}
            </div>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, fontFamily: "var(--font-mono)" }}>
              {item.value}
            </div>
          </Card>
        ))}
      </div>
      <div
        style={{
          background: "var(--color-warning-soft)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-3)",
          fontSize: "var(--font-size-sm)",
          color: "var(--color-warning)",
        }}
      >
        No hardware execution. This is a deterministic preview only.
      </div>
    </Panel>
  );
}

function SafetyPreviewPanel({ preview }: { preview: SafetyPreview }) {
  const decisionBadge =
    preview.decision === "allow" ? (
      <Badge variant="success">ALLOW</Badge>
    ) : preview.decision === "allow_with_warnings" ? (
      <Badge variant="warning">ALLOW WITH WARNINGS</Badge>
    ) : (
      <Badge variant="danger">REJECT</Badge>
    );

  return (
    <Panel title="Safety Report Preview">
      <div
        style={{
          display: "flex",
          gap: "var(--space-4)",
          marginBottom: "var(--space-4)",
          flexWrap: "wrap",
        }}
      >
        <Card>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
            Decision
          </div>
          <div>{decisionBadge}</div>
        </Card>
        <Card>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
            Checks
          </div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, fontFamily: "var(--font-mono)" }}>
            {preview.passed_count}/{preview.total_checks}
          </div>
        </Card>
        <Card>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
            Errors
          </div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, fontFamily: "var(--font-mono)", color: "var(--color-danger)" }}>
            {preview.errors_count}
          </div>
        </Card>
        <Card>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
            Warnings
          </div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, fontFamily: "var(--font-mono)", color: "var(--color-warning)" }}>
            {preview.warnings_count}
          </div>
        </Card>
        <Card>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
            Operator Approval
          </div>
          <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>
            {preview.operator_approval_required ? "Required" : "Not required"}
          </div>
        </Card>
      </div>

      <div
        style={{
          background: "var(--color-surface)",
          borderRadius: "var(--radius-md)",
          border: "1px solid var(--color-border)",
          overflow: "hidden",
        }}
      >
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ background: "var(--table-header-bg)" }}>
              {["Check", "Status", "Detail"].map((h) => (
                <th
                  key={h}
                  scope="col"
                  style={{
                    padding: "var(--table-density-cell-padding)",
                    textAlign: "left",
                    fontWeight: 600,
                    borderBottom: "1px solid var(--color-border)",
                  }}
                >
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {preview.findings.map((f, i) => (
              <FindingRow key={i} finding={f} />
            ))}
          </tbody>
        </table>
      </div>
    </Panel>
  );
}

function FindingRow({ finding }: { finding: SafetyFinding }) {
  const statusBadge = finding.passed ? (
    <Badge variant="success">PASS</Badge>
  ) : finding.severity === "error" ? (
    <Badge variant="danger">FAIL</Badge>
  ) : (
    <Badge variant="warning">WARN</Badge>
  );

  return (
    <tr style={{ borderBottom: "1px solid var(--color-border)" }}>
      <td style={{ padding: "var(--table-density-cell-padding)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)" }}>
        {finding.check}
      </td>
      <td style={{ padding: "var(--table-density-cell-padding)" }}>{statusBadge}</td>
      <td style={{ padding: "var(--table-density-cell-padding)", color: "var(--color-text-muted)", fontSize: "var(--font-size-xs)" }}>
        {finding.detail}
      </td>
    </tr>
  );
}

function CommandPlanPreviewPanel({ preview }: { preview: CommandPlanPreview }) {
  return (
    <Panel title="Command Plan Preview">
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gap: "var(--space-4)",
          marginBottom: "var(--space-4)",
        }}
      >
        {[
          { label: "Total Commands", value: String(preview.total_commands) },
          { label: "SMB Set", value: String(preview.smb_set_count) },
          { label: "SMB Query", value: String(preview.smb_query_count) },
          { label: "OE Commands", value: String(preview.oe_count) },
          { label: "Shutdown", value: String(preview.shutdown_count) },
          { label: "Safety Relevant", value: String(preview.safety_relevant_count) },
        ].map((item) => (
          <Card key={item.label}>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
              {item.label}
            </div>
            <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, fontFamily: "var(--font-mono)" }}>
              {item.value}
            </div>
          </Card>
        ))}
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(3, 1fr)",
          gap: "var(--space-4)",
        }}
      >
        <Card>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
            Forbidden Commands
          </div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, fontFamily: "var(--font-mono)", color: preview.forbidden_count === 0 ? "var(--color-success)" : "var(--color-danger)" }}>
            {preview.forbidden_count}
          </div>
        </Card>
        <Card>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
            Internal Sweep
          </div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, fontFamily: "var(--font-mono)", color: preview.internal_sweep_used ? "var(--color-danger)" : "var(--color-success)" }}>
            {preview.internal_sweep_used ? "YES" : "NO"}
          </div>
        </Card>
        <Card>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--color-text-muted)", marginBottom: "var(--space-1)" }}>
            Magnetic Commands
          </div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, fontFamily: "var(--font-mono)", color: preview.magnetic_commands === 0 ? "var(--color-success)" : "var(--color-danger)" }}>
            {preview.magnetic_commands}
          </div>
        </Card>
      </div>
    </Panel>
  );
}

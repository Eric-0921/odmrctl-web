export interface Recipe {
  schema_version: string;
  kind: string;
  id: string;
  name: string;
  created_by: string;
  created_at: string;
  description: string;
  station_id: string;
  intent: {
    experiment_type: string;
    description: string;
  };
  profiles: string[];
  blocks: string[];
  sweeps: Array<{
    sweep_id: string;
    axis: string;
    start: number;
    stop: number;
    step: number;
    order: string;
  }>;
  acquisition: {
    device_id: string;
    channel: string;
    readout: string[];
    pre_discard_ms: number;
    window_ms: number;
    average: {
      mode: string;
      repeat: number;
    };
  };
  timing: {
    default_dwell_ms: number;
    default_settle_ms: number;
  };
  metadata: {
    sample_id: string;
    operator: string;
    notes: string;
  };
}

export interface DryRunStep {
  step_id: string;
  sweep_coordinate: Record<string, number>;
  device_actions: string[];
  estimated_duration_ms: number;
}

export interface DryRunPlan {
  schema_version: string;
  kind: string;
  id: string;
  resolved_recipe_id: string;
  summary: {
    step_count: number;
    estimated_duration_s: number;
    required_devices: string[];
    hazard_actions: number;
  };
  steps: DryRunStep[];
}

export interface SafetyFinding {
  severity: string;
  code?: string;
  message: string;
  step_id?: string;
  device_id?: string;
}

export interface SafetyReport {
  schema_version: string;
  kind: string;
  id: string;
  resolved_recipe_id: string;
  decision: "allow" | "reject" | "unknown";
  summary: {
    checked_steps: number;
    checked_actions: number;
    info_count: number;
    warning_count: number;
    error_count: number;
  };
  findings: SafetyFinding[];
}

export interface RunEvent {
  schema_version: string;
  kind: string;
  run_id: string;
  event_id: string;
  timestamp_unix_ms: number;
  level: string;
  event_type: string;
  step_id?: string;
  message: string;
}

export interface RawIndexEntry {
  schema_version: string;
  kind: string;
  run_id: string;
  stream_id: string;
  offset_bytes: number;
  length_bytes: number;
  timestamp_unix_ms: number;
  step_id: string;
}

export interface RunManifest {
  schema_version: string;
  kind: string;
  run_id: string;
  created_at_unix_ms: number;
  artifact_paths: Record<string, string>;
  recipe_hash: string;
  resolved_recipe_id: string;
  safety_report_id: string;
}

export interface RawArtifactSummary {
  filename: string;
  sizeBytes: number;
  parsedByGui: boolean;
  note: string;
}

export interface RunSummary {
  runId: string;
  name: string;
  mode: "mock";
  stepCount: number;
  estimatedDurationMs: number;
  eventCount: number;
  artifactCount: number;
  safetyDecision: "allow" | "reject" | "unknown";
  requiredDevices: string[];
  recipeName: string;
  recipeHash: string;
}

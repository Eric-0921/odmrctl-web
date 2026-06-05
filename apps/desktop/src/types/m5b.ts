export interface M5bRecipe {
  schema_version: string;
  kind: string;
  id: string;
  description: string;
  station_ref: string;
  physical_response_required: boolean;
  devices: Record<string, { device_id: string; required: boolean; enabled?: boolean }>;
  fixed_params: Record<string, unknown>;
  sweeps: M5bSweep[];
  sweep_order: string[];
  acquisition_policy: Record<string, unknown>;
  safety: Record<string, boolean>;
}

export interface M5bSweep {
  sweep_id: string;
  device?: string;
  axis_group?: string;
  type?: string;
  unit?: string;
  axis?: string;
  axes?: Record<string, { value?: number; values?: number[] }>;
  values?: number[];
}

export interface M5bResolvedRecipe {
  schema_version: string;
  kind: string;
  id: string;
  description: string;
  source_recipe_id: string;
  source_recipe_hash: string;
  station_id: string;
  safety_report_id: string;
  estimated_duration_s: number;
  step_count: number;
  steps: M5bResolvedStep[];
}

export interface M5bResolvedStep {
  step_id: string;
  phase: "setup" | "measure" | "cleanup";
  point_index?: number;
  sweep_coordinates?: Record<string, number>;
  target_device_state: Record<string, unknown>;
  acquisition: { enabled: boolean; [key: string]: unknown };
  traceability: Record<string, unknown>;
}

export interface M5bSafetyReport {
  schema_version: string;
  kind: string;
  id: string;
  resolved_recipe_id: string;
  decision: "allow" | "deny";
  requires_operator_approval: boolean;
  physical_response_required: boolean;
  summary: {
    checked_steps: number;
    checked_actions: number;
    info_count: number;
    warning_count: number;
    error_count: number;
  };
  checks: M5bSafetyCheck[];
  warnings: string[];
  errors: string[];
}

export interface M5bSafetyCheck {
  check: string;
  status: "pass" | "warn" | "fail";
  message: string;
  value?: number;
  limit?: number;
}

export interface M5bDryRunPlan {
  schema_version: string;
  kind: string;
  id: string;
  resolved_recipe_id: string;
  summary: {
    step_count: number;
    total_points: number;
    expected_frames: number;
    estimated_duration_s: number;
    required_devices: string[];
    hazard_actions: number;
    outer_sweep: string;
    inner_sweep: string;
  };
  phases: M5bDryRunPhase[];
  operator_approval_required: boolean;
}

export interface M5bDryRunPhase {
  phase: string;
  description: string;
  hazard_note?: string;
  steps: { step_id: string; description: string }[];
}

export interface M5bStation {
  schema_version: string;
  kind: string;
  id: string;
  name: string;
  description: string;
  devices: M5bStationDevice[];
  safety: Record<string, unknown>;
}

export interface M5bStationDevice {
  device_id: string;
  kind: string;
  transport: string;
  address: string | null;
  expected_sn: string | null;
  timeout_ms: number;
  profile_ref?: string;
}

export interface M5bDeviceProfile {
  schema_version: string;
  kind: string;
  device_type: string;
  id: string;
  description: string;
  [key: string]: unknown;
}

// M4.1 TypeScript types for M3.4 recipe preview.
// Field names match the Rust M3_4Recipe struct and JSON shape exactly.

export interface M34Recipe {
  schema_version: string;
  kind: string;
  id: string;
  description?: string;
  devices: {
    smb100a: { device_id: string; mode?: string };
    oe1022d: { device_id: string; mode?: string };
    magnetic: { in_scope: boolean };
  };
  rf: {
    start_hz: number;
    stop_hz: number;
    points: number;
    power_dbm: number;
    max_power_dbm: number;
  };
  modulation: {
    fm_source?: string;
    fm_deviation_hz: number;
    max_fm_deviation_hz: number;
    internal_lf?: {
      enabled: boolean;
      frequency_hz: number;
      shape: string;
      voltage_v: number;
      lf_output_enabled?: boolean;
    };
  };
  acquisition: {
    frames_per_step: number;
    repeat_count?: number;
    inter_frame_delay_ms?: number;
  };
  safety: {
    require_operator_approval?: boolean;
    no_internal_sweep?: boolean;
    no_csv?: boolean;
    no_gui?: boolean;
    no_magnetic?: boolean;
    physical_response_required?: boolean;
  };
}

export interface RecipeValidationResult {
  parseOk: boolean;
  parseError?: string;
  shapeOk: boolean;
  shapeErrors: string[];
  valueOk: boolean;
  valueErrors: string[];
  warnings: string[];
  recipe: M34Recipe | null;
}

export interface ResolvedPreview {
  step_count: number;
  frequencies: number[];
  total_frames: number;
  estimated_duration_s: number;
  device_list: string[];
  physical_response_required: boolean;
}

export interface DryRunPreview {
  step_count: number;
  total_frames: number;
  repeat_count: number;
  rf_points: number;
  estimated_duration_s: number;
  required_devices: string[];
  smb_set_count: number;
  smb_query_count: number;
  oe_frame_count: number;
}

export interface SafetyFinding {
  check: string;
  severity: "info" | "warning" | "error";
  passed: boolean;
  detail: string;
}

export interface SafetyPreview {
  decision: "allow" | "reject" | "allow_with_warnings";
  findings: SafetyFinding[];
  total_checks: number;
  passed_count: number;
  warnings_count: number;
  errors_count: number;
  operator_approval_required: boolean;
}

export interface CommandPlanPreview {
  total_commands: number;
  smb_set_count: number;
  smb_query_count: number;
  oe_count: number;
  shutdown_count: number;
  safety_relevant_count: number;
  forbidden_count: number;
  internal_sweep_used: boolean;
  magnetic_commands: number;
}

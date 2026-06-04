// M4.0 TypeScript types for M3.6 analysis artifacts.
// Field names match the Rust AnalysisData struct exactly for serde deserialization.

export interface SpectrumPoint {
  run_id: string;
  step_id: string;
  repeat_index: number;
  frequency_hz: number;
  frequency_verified_hz: number;
  b_x_mean_v: number;
  b_x_mean_mv: number;
  b_x_std_v: number;
  b_y_mean_v: number;
  b_y_mean_mv: number;
  b_y_std_v: number;
  frames_used: number;
  frames_parse_failed: number;
  step_passed: boolean;
  quality_flags: string[];
}

export interface FrequencyGroup {
  frequency_hz: number;
  point_count: number;
  total_frames_used: number;
  frames_parse_failed: number;
  contributing_run_ids: string[];
  b_x_mean_v: number;
  b_x_mean_mv: number;
  b_x_std_v: number;
  b_x_std_mv: number;
  b_x_min_v: number;
  b_x_min_mv: number;
  b_x_max_v: number;
  b_x_max_mv: number;
  b_y_mean_v: number;
  b_y_mean_mv: number;
  b_y_std_v: number;
  b_y_std_mv: number;
  b_y_min_v: number;
  b_y_min_mv: number;
  b_y_max_v: number;
  b_y_max_mv: number;
}

export interface RunOverlaySummary {
  schema_version: string;
  kind: string;
  frequency_count: number;
  generated_at: string;
  frequencies: FrequencyGroup[];
}

export interface OdmrLikeAnalysisSummary {
  schema_version: string;
  kind: string;
  source_run_ids: string[];
  frequency_count: number;
  point_count: number;
  frames_used: number;
  frames_parse_failed: number;
  parse_failure_rate: number;
  all_runs_passed: boolean;
  all_safe_states_confirmed: boolean;
  no_csv: boolean;
  no_magnetic: boolean;
  quality_flags_passed: boolean;
  odmr_dip_detected: boolean;
  physical_odmr_response_required: boolean;
  contrast_estimate_b_x_v: number | null;
  contrast_estimate_b_x_mv: number | null;
  contrast_estimate_b_y_v: number | null;
  contrast_estimate_b_y_mv: number | null;
  oe1022d_display_idn_by_run: Record<string, string>;
  generated_at: string;
}

export interface QualityFlags {
  schema_version: string;
  kind: string;
  passed: boolean;
  missing_artifact: boolean;
  missing_artifact_details: Record<string, string[]>;
  failed_run: boolean;
  failed_run_ids: string[];
  parse_failures: boolean;
  parse_failure_count: number;
  audit_mismatch: boolean;
  audit_mismatch_run_ids: string[];
  unsafe_final_state: boolean;
  unsafe_final_state_run_ids: string[];
  csv_present: boolean;
  csv_present_details: Record<string, string[]>;
  magnetic_command_present: boolean;
  magnetic_command_details: Record<string, number>;
  frequency_grid_mismatch: boolean;
  empty_signal_series: boolean;
  generated_at: string;
}

export interface ExportManifestFile {
  relative_path: string;
  sha256: string;
  size_bytes: number;
}

export interface ExportManifest {
  schema_version: string;
  kind: string;
  source_run_ids: string[];
  generated_at: string;
  files: ExportManifestFile[];
}

export interface AnalysisData {
  spectrum_points: SpectrumPoint[];
  run_overlay_summary: RunOverlaySummary;
  analysis_summary: OdmrLikeAnalysisSummary;
  quality_flags: QualityFlags;
  export_manifest: ExportManifest | null;
  warnings: string[];
}

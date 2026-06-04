// M5A TypeScript types for combined run artifact viewer.
// Field names match the Rust types exactly for serde deserialization.

// ---------------------------------------------------------------------------
// Preflight types (from common_preflight)
// ---------------------------------------------------------------------------

export interface SafeState {
  confirmed: boolean;
  rf_output: string | null;
  modulation: string | null;
  fm: string | null;
  magnetic_output: string | null;
  magnetic_current_ma: number | null;
}

export interface DevicePreflightReport {
  device_id: string;
  kind: string;
  reachability: boolean;
  identity_raw: string | null;
  identity_display: string | null;
  error_queue: string[];
  safe_state: SafeState | null;
  warnings: string[];
  commands_sent: string[] | null;
  laser_on_sent: boolean | null;
  nonzero_power_sent: boolean | null;
}

export interface DeviceLockStatus {
  device_id: string;
  acquired: boolean;
  lock_file: string;
  pid: number | null;
  error: string | null;
}

export interface StationPreflightReport {
  schema_version: string;
  generated_at: string;
  station_profile: string;
  all_devices_reachable: boolean;
  all_identities_verified: boolean;
  all_safe_states_confirmed: boolean;
  operator_approved: boolean;
  elapsed_ms: number;
  devices: DevicePreflightReport[];
  lock_status: DeviceLockStatus[];
}

// ---------------------------------------------------------------------------
// Combined run report types (from rf_mag_oe_minimal_run)
// ---------------------------------------------------------------------------

export interface RfReportSection {
  requested_frequency_hz: number;
  requested_power_dbm: number;
  readback_frequency_hz: number | null;
  readback_power_dbm: number | null;
  rf_on_window_start_unix_ms: number | null;
  rf_on_window_end_unix_ms: number | null;
  rf_final_off: boolean;
}

export interface MagReportSection {
  axis_id: string;
  expected_sn: string;
  observed_sn: string;
  zero_readback_current_ma: number;
  zero_readback_std_ma: number;
  commanded_recur_current_ma: number;
  measured_recur_current_ma: number;
  measured_recur_field_nt: number;
  current_error_ma: number;
  mag_final_output_off: boolean;
  mag_final_current_zero: boolean;
  mag_final_local_requested: boolean;
}

export interface OeReportSection {
  frames_requested: number;
  frames_acquired: number;
  raw_bin_bytes: number;
  frame_size_bytes: number;
  parse_failures: number;
  timeout_count: number;
}

export interface TimelineReportSection {
  rf_on_before_oe_capture: boolean;
  mag_hold_before_oe_capture: boolean;
  oe_capture_completed_before_cleanup: boolean;
  cleanup_completed: boolean;
}

export interface CombinedRunReport {
  schema_version: string;
  run_id: string;
  passed: boolean;
  rf: RfReportSection;
  magnetic: MagReportSection;
  oe: OeReportSection;
  timeline: TimelineReportSection;
  errors: string[];
}

// ---------------------------------------------------------------------------
// Event and audit types
// ---------------------------------------------------------------------------

export interface CombinedRunEvent {
  event_type: string;
  timestamp_unix_ms: number;
  device_id?: string;
  detail?: string;
}

export interface CommandAuditEntry {
  seq: number;
  timestamp_unix_ms: number;
  device_id: string;
  command: string;
  command_class: string;
  allowed: boolean;
  sent_to_transport: boolean;
  rejection_reason?: string;
  response_preview?: string;
  transport_error?: string;
  safety_relevant: boolean;
}

export interface FrameIndexEntry {
  frame_index: number;
  length: number;
  offset: number;
  timestamp_unix_ms: number;
}

export interface FrameSummaryEntry {
  elapsed_ms: number;
  frame_index: number;
  size_bytes: number;
}

// ---------------------------------------------------------------------------
// Aggregate M5A run data (returned by Tauri command)
// ---------------------------------------------------------------------------

export interface M5aRunData {
  preflight: StationPreflightReport | null;
  combined_run_report: CombinedRunReport | null;
  events: CombinedRunEvent[];
  smb_audit: CommandAuditEntry[];
  maynuo_audit: CommandAuditEntry[];
  oe_audit: CommandAuditEntry[];
  frame_index: FrameIndexEntry[];
  frame_summary: FrameSummaryEntry[];
  warnings: string[];
}

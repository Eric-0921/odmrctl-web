// Mirrors of odmr-preflight Rust types for M5C-A Device Workbench.

export interface DeviceConfig {
  device_id: string;
  kind: string;
  transport: string;
  address: string;
  expected_sn?: string;
  timeout_ms?: number;
}

export interface StationProfile {
  name: string;
  devices: DeviceConfig[];
}

export interface SafeState {
  confirmed: boolean;
  rf_output?: string;
  modulation?: string;
  fm?: string;
  magnetic_output?: string;
  magnetic_current_ma?: number;
}

export interface DevicePreflightReport {
  device_id: string;
  kind: string;
  reachability: boolean;
  identity_raw?: string;
  identity_display?: string;
  error_queue: string[];
  safe_state?: SafeState;
  warnings: string[];
  commands_sent?: string[];
  laser_on_sent?: boolean;
  nonzero_power_sent?: boolean;
}

export interface DeviceLockStatus {
  device_id: string;
  acquired: boolean;
  lock_file: string;
  pid?: number;
  error?: string;
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
// Device Panel status types
// ---------------------------------------------------------------------------

export interface Smb100aStatus {
  connected: boolean;
  frequency_hz?: number;
  power_dbm?: number;
  output_on?: boolean;
  modulation_on?: boolean;
  fm_enabled?: boolean;
  fm_source?: string;
  fm_mode?: string;
  fm_deviation_hz?: number;
  lf_frequency_hz?: number;
  lf_voltage_v?: number;
  lf_output_on?: boolean;
  lf_shape?: string;
  lf_impedance?: string;
  error_queue: string[];
  last_readback_time: string;
}

export interface Oe1022dStatus {
  connected: boolean;
  reference_source?: string;
  ref_slope?: string;
  phase_deg?: number;
  time_constant_s?: number;
  filter_slope_db_oct?: number;
  input_source?: string;
  input_grounding?: string;
  input_coupling?: string;
  input_notch?: string;
  dynamic_reserve?: string;
  sensitivity_index?: number;
  sync_filter_on?: boolean;
  input_overload?: boolean;
  gain_overload?: boolean;
  pll_locked?: boolean;
  last_readback_time: string;
}

/// Magnetic status — matches original GUI logic:
/// - zero_bias_a:      user-set zero-field bias current (A)
/// - recur_current_a:  reproduction current added when lock_zero=ON
/// - recur_mag_nt:     reproduction magnetic field (nT)
/// - lock_zero:        when true, zero_bias is frozen and recur is added
/// - total_command_a:  zero_bias + (lock_zero ? recur_current : 0)
/// - coil_constant_nt_per_ma: nT per mA for Mag ↔ Current conversion
export interface MagneticStatus {
  connected: boolean;
  device_id: string;
  output_on: boolean;
  zero_bias_a: number;
  recur_current_a: number;
  recur_mag_nt: number;
  lock_zero: boolean;
  total_command_a: number;
  measured_current_a?: number;
  coil_constant_nt_per_ma: number;
  identity?: string;
  error_queue?: string;
  last_readback_time: string;
}

export interface RuntimeZeroAxisBaseline {
  device_id: string;
  axis: string;
  identity?: string;
  zero_samples_a: number[];
  zero_mean_a: number;
  zero_std_a: number;
  coil_constant_nt_per_ma: number;
}

export interface RuntimeZeroBaseline {
  schema_version: string;
  kind: string;
  session_id: string;
  locked_at: string;
  axes: Record<string, RuntimeZeroAxisBaseline>;
}

export interface MagneticVectorApplyResult {
  b_target_nt: [number, number, number];
  runtime_zero_baseline?: RuntimeZeroBaseline;
  axes: MagneticStatus[];
}

export interface MagneticAxisPackageStatus {
  axis: string;
  device_id: string;
  address: string;
  expected_sn: string;
  observed_idn?: string | null;
  connected: boolean;
  sn_match?: boolean | null;
  coil_constant_nt_per_ma: number;
  zero_bias_a: number;
  runtime_zero_mean_a?: number | null;
  runtime_zero_std_a?: number | null;
  lock_zero: boolean;
  recur_mag_nt: number;
  recur_current_a: number;
  total_command_a: number;
  measured_total_current_a?: number | null;
  reconstructed_recur_mag_nt?: number | null;
  output_on: boolean;
  max_current_a: number;
  blocked_reasons: string[];
  last_readback_time?: string | null;
}

export interface MagneticXyzPackageStatus {
  package_id: string;
  calibration_source: string;
  target_b_nt: [number, number, number];
  runtime_zero_baseline?: RuntimeZeroBaseline | null;
  axes: MagneticAxisPackageStatus[];
  ready_to_apply: boolean;
  blocked_reasons: string[];
}

export interface LaserStatus {
  connected: boolean;
  power_setpoint_mw: number;
  enabled: boolean;
  note: string;
  last_command_time: string;
}

export interface WorkbenchSnapshot {
  profile_loaded: boolean;
  profile_name?: string;
  preflight_passed: boolean;
  locks_held: string[];
  report?: StationPreflightReport;
  profile_addresses: Record<string, string>;
}

export interface SerialPortInfo {
  port_name: string;
  port_type: string;
}

export interface IdentifiedSerialDevice {
  port: string;
  detected_kind: string;
  idn?: string | null;
  serial_number?: string | null;
  confidence: string;
  suggested_role?: string | null;
  status: string;
}

export interface SerialIdentifyReport {
  ports: SerialPortInfo[];
  devices: IdentifiedSerialDevice[];
  warnings: string[];
}

export interface DeviceRoleRequest {
  device_id: string;
  kind: string;
  expected_sn?: string | null;
}

export interface AutoBoundDevice {
  device_id: string;
  kind: string;
  address?: string | null;
  idn?: string | null;
  serial_number?: string | null;
  confidence: string;
  status: string;
}

export interface AutoBindReport {
  bound: AutoBoundDevice[];
  blocked: string[];
}

export interface DiscoveredDevice {
  transport: string;
  address: string;
  detected_kind: string;
  idn?: string | null;
  serial_number?: string | null;
  model?: string | null;
  confidence: string;
  suggested_role?: string | null;
  status: string;
}

export interface DeviceDiscoveryReport {
  serial_ports: SerialPortInfo[];
  tcp_targets: string[];
  usb_resources: string[];
  devices: DiscoveredDevice[];
  warnings: string[];
}

export interface DeviceProbeRequest {
  requested_kinds: string[];
  smb100a_tcp_targets: string[];
  enable_usb_probe: boolean;
}

export interface ExperimentPlanSummary {
  schema_version: string;
  kind: string;
  id: string;
  station_ref?: string;
  preset_refs: unknown;
  field_point_count: number;
  rf_point_count: number;
  estimated_measurements: number;
  require_zero_lock: boolean;
  warnings: string[];
  raw: unknown;
}

export interface ResolvedMagneticPoint {
  point_index: number;
  b_target_nt: [number, number, number];
  computed_total_current_a: Record<string, number>;
  recurrent_current_a: Record<string, number>;
  zero_baseline_ref?: string | null;
  coil_constant_source: string;
}

export interface ResolvedPlanPreview {
  kind: string;
  executable: boolean;
  blocked_reasons: string[];
  zero_baseline?: RuntimeZeroBaseline;
  magnetic_points: ResolvedMagneticPoint[];
  rf_point_count: number;
  estimated_measurements: number;
}

export interface ExperimentRunReadiness {
  kind: string;
  ready_for_preview_execution: boolean;
  ready_for_hardware_execution: boolean;
  blocked_reasons: string[];
  hardware_blocked_reasons: string[];
  warnings: string[];
  step_count: number;
  rf_point_count: number;
  estimated_measurements: number;
  estimated_duration_s?: number | null;
  require_zero_lock: boolean;
  zero_baseline_present: boolean;
  connected_devices: string[];
  required_devices: string[];
}

export interface ExperimentPlanRunStatus {
  kind: string;
  run_id: string;
  mode: string;
  state: string;
  started_at: string;
  finished_at?: string | null;
  run_directory?: string | null;
  step_count: number;
  rf_point_count: number;
  estimated_measurements: number;
  estimated_duration_s?: number | null;
  steps_completed: number;
  current_step_index?: number | null;
  current_step_id?: string | null;
  current_b_nt?: [number, number, number] | null;
  current_phase?: string | null;
  smb_sweep_running: boolean;
  oe_frames_captured: number;
  cleanup_state?: string | null;
  recent_error?: string | null;
  blocked_reasons: string[];
  warnings: string[];
  artifact_paths: Record<string, string>;
}

export interface MagneticPointProjection {
  point_index: number;
  group_id?: string | null;
  bx_nt: number;
  by_nt: number;
  bz_nt: number;
  source: string;
}

export interface Smb100aRfPointProjection {
  point_index: number;
  frequency_hz: number;
  power_dbm?: number | null;
  dwell_ms?: number | null;
  fm_enabled?: boolean | null;
  lf_frequency_hz?: number | null;
  modulation_on?: boolean | null;
  sweep_output_start_v?: number | null;
  sweep_output_stop_v?: number | null;
}

export interface LaserProjection {
  mode: string;
  power_mw?: number | null;
  enabled?: boolean | null;
  settle_ms?: number | null;
}

export interface Oe1022dProjection {
  frames_per_point?: number | null;
  pre_discard_ms?: number | null;
  pre_start_ms?: number | null;
  post_stop_ms?: number | null;
  time_constant_s?: number | null;
  filter_slope_db_oct?: number | null;
  reference_source?: string | null;
  acquisition: unknown;
}

export interface CombinationPreviewRow {
  row_index: number;
  magnetic_point_index: number;
  rf_point_index: number;
  bx_nt: number;
  by_nt: number;
  bz_nt: number;
  frequency_hz: number;
  laser_mode: string;
  oe_frames_per_point?: number | null;
}

export interface DevicePanelCatalog {
  device: string;
  panel_group: string;
  field_id: string;
  label_cn: string;
  unit?: string | null;
  field_type: string;
  allowed_values: string[];
  display_values: CatalogDisplayValue[];
  unit_options: string[];
  default_unit?: string | null;
  default_value?: unknown;
  default_value_si?: unknown;
  safe_value?: unknown;
  safe_value_si?: unknown;
  query_command?: string | null;
  set_command?: string | null;
  remote_code?: string | null;
  ui_location: string;
  channel_scope?: string | null;
  enabled_when?: string | null;
  disabled_reason_cn?: string | null;
  write_policy: string;
  json_path: string;
}

export interface CatalogDisplayValue {
  value: string;
  label_cn: string;
  status_color?: string | null;
}

export interface DeviceDefaultPackage {
  device: string;
  package_id: string;
  label_cn: string;
  source: string;
  risk_level: string;
  values: unknown;
  values_si: unknown;
  note_cn: string;
  apply_target: string;
}

export interface ExperimentStepProjection {
  step_index: number;
  step_id: string;
  group_id?: string | null;
  bx_nt: number;
  by_nt: number;
  bz_nt: number;
  rf_start_hz?: number | null;
  rf_stop_hz?: number | null;
  rf_step_hz?: number | null;
  smb100a_frequency_hz?: number | null;
  smb100a_power_dbm?: number | null;
  smb100a_fm_enabled?: boolean | null;
  smb100a_lf_frequency_hz?: number | null;
  smb100a_rf_sweep_summary: string;
  smb100a_sweep_output_start_v?: number | null;
  smb100a_sweep_output_stop_v?: number | null;
  laser_power_mw?: number | null;
  laser_enabled?: boolean | null;
  oe1022d_summary: string;
  oe_pre_start_ms?: number | null;
  oe_post_stop_ms?: number | null;
  oe_ch_a_time_constant_s?: number | null;
  oe_ch_a_filter_slope_db_oct?: number | null;
  oe_ch_a_dynamic_reserve?: string | null;
  oe_ch_a_sensitivity?: string | null;
  oe_ch_b_time_constant_s?: number | null;
  oe_ch_b_filter_slope_db_oct?: number | null;
  oe_ch_b_dynamic_reserve?: string | null;
  oe_ch_b_sensitivity?: string | null;
  dwell_ms?: number | null;
  estimated_duration_s?: number | null;
  executable: boolean;
  blocked_reasons: string[];
}

export interface ExperimentPlanProjection {
  kind: string;
  panel_catalogs: DevicePanelCatalog[];
  default_packages: DeviceDefaultPackage[];
  step_rows: ExperimentStepProjection[];
  step_row_count: number;
  preview_limit: number;
  truncated: boolean;
  magnetic_points: MagneticPointProjection[];
  smb100a_rf_points: Smb100aRfPointProjection[];
  laser_rows: LaserProjection[];
  oe1022d_rows: Oe1022dProjection[];
  combination_preview: CombinationPreviewRow[];
  estimated_measurements: number;
  estimated_duration_s?: number | null;
  warnings: string[];
}

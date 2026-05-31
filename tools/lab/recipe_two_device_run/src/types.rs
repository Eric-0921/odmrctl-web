//! M3.4 types: recipe, resolved recipe, dry run, safety, command plan, audit comparison, run results.
//! All structs derive Serialize, Deserialize, Debug, Clone.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Recipe types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4Recipe {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub devices: RecipeDevices,
    pub rf: RecipeRfConfig,
    pub modulation: RecipeModulationConfig,
    pub acquisition: RecipeAcquisitionConfig,
    pub safety: RecipeSafetyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDevices {
    pub smb100a: RecipeDeviceRef,
    pub oe1022d: RecipeDeviceRef,
    pub magnetic: RecipeMagneticRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDeviceRef {
    pub device_id: String,
    #[serde(default)]
    pub mode: String, // "real_or_fake_by_runtime"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeMagneticRef {
    pub in_scope: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeRfConfig {
    pub start_hz: f64,
    pub stop_hz: f64,
    pub points: u64,
    pub power_dbm: f64,
    pub max_power_dbm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeModulationConfig {
    #[serde(default = "default_fm_source")]
    pub fm_source: String,
    pub fm_deviation_hz: f64,
    pub max_fm_deviation_hz: f64,
    #[serde(default)]
    pub internal_lf: Option<RecipeInternalLf>,
}

fn default_fm_source() -> String {
    "INT".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInternalLf {
    pub enabled: bool,
    pub frequency_hz: f64,
    pub shape: String,
    pub voltage_v: f64,
    #[serde(default)]
    pub lf_output_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeAcquisitionConfig {
    pub frames_per_step: u64,
    #[serde(default = "default_repeat_count")]
    pub repeat_count: u64,
    #[serde(default = "default_inter_frame_delay_ms")]
    pub inter_frame_delay_ms: u64,
}

fn default_repeat_count() -> u64 {
    2
}
fn default_inter_frame_delay_ms() -> u64 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSafetyConfig {
    #[serde(default = "default_true")]
    pub require_operator_approval: bool,
    #[serde(default = "default_true")]
    pub no_internal_sweep: bool,
    #[serde(default = "default_true")]
    pub no_csv: bool,
    #[serde(default = "default_true")]
    pub no_gui: bool,
    #[serde(default = "default_true")]
    pub no_magnetic: bool,
    #[serde(default)]
    pub physical_response_required: bool,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Resolved recipe
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4ResolvedRecipe {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub source_recipe_id: String,
    pub source_recipe_hash: String,
    pub estimated_duration_s: f64,
    pub safety_report_id: Option<String>,
    pub total_steps: u64,
    pub steps: Vec<M3_4ResolvedStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4ResolvedStep {
    pub step_id: String,
    pub repeat_index: u64,
    pub point_index: u64,
    pub total_points: u64,
    pub frequency_hz: f64,
    pub rf_power_dbm: f64,
    pub fm_deviation_hz: f64,
    pub fm_on: bool,
    pub mod_on: bool,
    pub lf_enabled: bool,
    pub frames_to_acquire: u64,
    pub estimated_duration_ms: f64,
    pub expected_smb_commands: Vec<String>,
}

// ---------------------------------------------------------------------------
// Dry run plan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4DryRunPlan {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub resolved_recipe_id: String,
    pub summary: M3_4DryRunSummary,
    pub steps: Vec<M3_4DryRunStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4DryRunSummary {
    pub step_count: u64,
    pub total_frames: u64,
    pub repeat_count: u64,
    pub rf_points: u64,
    pub estimated_duration_s: f64,
    pub required_devices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4DryRunStep {
    pub step_id: String,
    pub repeat_index: u64,
    pub frequency_hz: f64,
    pub device_actions: Vec<String>,
    pub frames_to_acquire: u64,
    pub estimated_duration_ms: f64,
}

// ---------------------------------------------------------------------------
// Safety report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SafetyDecision {
    Allow,
    AllowWithWarnings,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4SafetyReport {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub resolved_recipe_id: String,
    pub decision: SafetyDecision,
    pub summary: M3_4SafetySummary,
    pub findings: Vec<M3_4SafetyFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4SafetySummary {
    pub total_checks: u64,
    pub passed: u64,
    pub warnings: u64,
    pub errors: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4SafetyFinding {
    pub check: String,
    pub severity: String, // "info", "warning", "error"
    pub passed: bool,
    pub detail: String,
}

// ---------------------------------------------------------------------------
// Command plan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPlanEntry {
    pub sequence_index: u64,
    pub step_id: String,
    pub repeat_index: u64,
    pub device_id: String,
    pub command: String,
    pub command_class: String, // "set", "query", "shutdown"
    pub safety_relevant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPlanSummary {
    pub schema_version: String,
    pub kind: String,
    pub total_commands: u64,
    pub set_commands: u64,
    pub query_commands: u64,
    pub shutdown_commands: u64,
    pub safety_relevant_commands: u64,
}

// ---------------------------------------------------------------------------
// Command audit comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAuditComparison {
    pub schema_version: String,
    pub kind: String,
    pub passed: bool,
    pub expected_command_count: u64,
    pub actual_command_count: u64,
    pub missing_expected_commands: Vec<String>,
    pub unexpected_actual_commands: Vec<String>,
    pub forbidden_actual_commands: Vec<String>,
    pub allowed_extra_queries: Vec<String>,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Command audit entry (M3.4 version)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4CommandAuditEntry {
    pub timestamp_unix_ms: u64,
    pub device_id: String,
    pub command: String,
    pub command_class: String,
    pub allowed: bool,
    pub sent_to_transport: bool,
    pub rejection_reason: Option<String>,
    pub response_preview: Option<String>,
    pub transport_error: Option<String>,
    pub safety_relevant: bool,
}

// ---------------------------------------------------------------------------
// Run result types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3_4RunResult {
    pub schema_version: String,
    pub kind: String,
    pub run_id: String,
    pub mode: String,
    pub recipe_id: String,
    pub resolved_recipe_id: String,
    pub passed: bool,
    pub steps_completed: u64,
    pub total_steps: u64,
    pub frames_requested: u64,
    pub frames_captured: u64,
    pub frames_parsed: u64,
    pub frames_parse_failed: u64,
    pub parse_failure_rate: f64,
    pub final_rf_off: bool,
    pub final_mod_off: bool,
    pub final_fm_off: bool,
    pub final_syst_err_clean: bool,
    pub command_audit_comparison_passed: bool,
    pub no_forbidden_commands_sent: bool,
    pub emergency_shutdown_triggered: bool,
    pub alignment_count: u64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RfStepSummaryEntry {
    pub step_id: String,
    pub repeat_index: u64,
    pub frequency_hz: f64,
    pub frequency_verified_hz: Option<f64>,
    pub rf_output_on: bool,
    pub frames_requested: u64,
    pub frames_captured: u64,
    pub frames_parsed: u64,
    pub frames_parse_failed: u64,
    pub step_passed: bool,
    pub b_x_mean: Option<f64>,
    pub b_x_std: Option<f64>,
    pub b_y_mean: Option<f64>,
    pub b_y_std: Option<f64>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStabilitySummary {
    pub schema_version: String,
    pub kind: String,
    pub run_id: String,
    pub frames_requested: u64,
    pub frames_captured: u64,
    pub frames_parsed: u64,
    pub frames_parse_failed: u64,
    pub parse_failure_rate: f64,
    pub steps_total: u64,
    pub steps_passed: u64,
    pub final_rf_off: bool,
    pub final_mod_off: bool,
    pub final_fm_off: bool,
    pub final_syst_err_clean: bool,
    pub emergency_shutdown_triggered: bool,
    pub no_forbidden_commands_sent: bool,
}

// ---------------------------------------------------------------------------
// Harness / replay types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessModeConfig {
    pub schema_version: String,
    pub kind: String,
    pub mode: String,
    pub recipe_id: String,
    pub use_deterministic_frames: bool,
    pub inject_parse_failures: bool,
    pub parse_failure_rate_target: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeDeviceTraceEntry {
    pub sequence: u64,
    pub device_id: String,
    pub command: String,
    pub response: String,
    pub timestamp_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySourceConfig {
    pub schema_version: String,
    pub kind: String,
    pub source_run_id: String,
    pub source_run_root: String,
    pub raw_bin_path: String,
    pub index_path: String,
    pub alignment_path: String,
    pub command_audit_path: String,
    pub step_plan_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayReport {
    pub schema_version: String,
    pub kind: String,
    pub source_run_id: String,
    pub replay_run_id: String,
    pub frames_replayed: u64,
    pub frames_parseable: u64,
    pub alignment_rebuilt: bool,
    pub statistics_rebuilt: bool,
    pub command_audit_compared: bool,
    pub passed: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DeterminismReport {
    pub schema_version: String,
    pub kind: String,
    pub run_ids: Vec<String>,
    pub recipe_hash: String,
    pub resolved_recipe_hash: String,
    pub dry_run_plan_hash: String,
    pub command_plan_hash: String,
    pub safety_report_decision_match: bool,
    pub identical: bool,
}

// ---------------------------------------------------------------------------
// Metadata / approval types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorApproval {
    pub schema_version: String,
    pub approved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub timestamp_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagneticNotInScope {
    pub schema_version: String,
    pub kind: String,
    pub message: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyBoundaryNote {
    pub schema_version: String,
    pub kind: String,
    pub no_internal_sweep: bool,
    pub no_csv: bool,
    pub no_gui: bool,
    pub no_magnetic: bool,
    pub rf_on_requires_approval: bool,
    pub physical_response_not_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationSnapshotQuality {
    pub schema_version: String,
    pub kind: String,
    pub smb_identity_verified: bool,
    pub oe_identity_verified: bool,
    pub eligible_for_extended_sweep: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Hash manifest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct HashManifest {
    pub schema_version: String,
    pub kind: String,
    pub run_id: String,
    pub recipe_hash: String,
    pub resolved_recipe_hash: String,
    pub dry_run_plan_hash: String,
    pub safety_report_hash: String,
    pub command_plan_hash: String,
    pub smb_snapshot_before_hash: Option<String>,
    pub smb_snapshot_after_hash: Option<String>,
}

// ---------------------------------------------------------------------------
// Audit report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub schema_version: String,
    pub kind: String,
    pub run_id: String,
    pub passed: bool,
    pub total_commands: u64,
    pub allowed_commands: u64,
    pub blocked_commands: u64,
    pub forbidden_commands_sent: u64,
    pub smb_set_count: u64,
    pub smb_query_count: u64,
    pub oe_command_count: u64,
    pub no_internal_sweep_commands: bool,
    pub no_magnetic_commands: bool,
    pub notes: Vec<String>,
}

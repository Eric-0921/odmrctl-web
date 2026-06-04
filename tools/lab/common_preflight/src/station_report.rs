use crate::types::StationPreflightReport;
use std::path::Path;

/// Write the station preflight report to a JSON file.
pub fn write_json<P: AsRef<Path>>(report: &StationPreflightReport, path: P) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| format!("serialize report: {}", e))?;
    std::fs::write(path, json)
        .map_err(|e| format!("write report: {}", e))
}

/// Write the station preflight report as a Markdown summary.
pub fn write_markdown<P: AsRef<Path>>(report: &StationPreflightReport, path: P) -> Result<(), String> {
    let mut lines = Vec::new();
    lines.push("# Station Preflight Report".to_string());
    lines.push("".to_string());
    lines.push(format!("- **Generated**: {}", report.generated_at));
    lines.push(format!("- **Profile**: {}", report.station_profile));
    lines.push(format!("- **Elapsed**: {} ms", report.elapsed_ms));
    lines.push("".to_string());

    let overall = if report.passed() { "✅ PASS" } else { "❌ FAIL" };
    lines.push(format!("## Overall: {}", overall));
    lines.push("".to_string());
    lines.push(format!("- All reachable: {}", report.all_devices_reachable));
    lines.push(format!("- All identities verified: {}", report.all_identities_verified));
    lines.push(format!("- All safe states confirmed: {}", report.all_safe_states_confirmed));
    lines.push(format!("- Operator approved: {}", report.operator_approved));
    lines.push("".to_string());

    lines.push("## Per-Device Results".to_string());
    lines.push("".to_string());
    lines.push("| Device | Kind | Reachable | Identity | Safe State | Warnings |".to_string());
    lines.push("|--------|------|-----------|----------|------------|----------|".to_string());

    for d in &report.devices {
        let id_short = d.identity_display.as_deref().unwrap_or("—");
        let id_str = if id_short.len() > 40 { &id_short[..40] } else { id_short };
        let safe = d.safe_state.as_ref().map(|s| if s.confirmed { "✅" } else { "❌" }).unwrap_or("—");
        let warn_count = d.warnings.len();
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            d.device_id, d.kind, if d.reachability { "✅" } else { "❌" },
            id_str, safe, warn_count
        ));
    }

    lines.push("".to_string());

    for d in &report.devices {
        if !d.error_queue.is_empty() || !d.warnings.is_empty() {
            lines.push(format!("### {} Details", d.device_id));
            lines.push("".to_string());
            if !d.error_queue.is_empty() {
                lines.push("**Error queue:**".to_string());
                for e in &d.error_queue {
                    lines.push(format!("- `{}`", e));
                }
                lines.push("".to_string());
            }
            if !d.warnings.is_empty() {
                lines.push("**Warnings:**".to_string());
                for w in &d.warnings {
                    lines.push(format!("- {}", w));
                }
                lines.push("".to_string());
            }
        }
    }

    std::fs::write(path, lines.join("\n"))
        .map_err(|e| format!("write markdown: {}", e))
}

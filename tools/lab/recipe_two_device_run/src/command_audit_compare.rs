//! Compare expected command plan vs actual command audit.

use crate::types::*;

/// Compare expected command plan entries with actual command audit entries.
pub fn compare_plan_vs_audit(
    expected: &[CommandPlanEntry],
    actual: &[M3_4CommandAuditEntry],
) -> CommandAuditComparison {
    let mut missing: Vec<String> = Vec::new();
    let mut unexpected: Vec<String> = Vec::new();
    let mut forbidden: Vec<String> = Vec::new();
    let mut allowed_extra: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    // Collect expected set commands in order (queries are flexible)
    let expected_sets: Vec<&CommandPlanEntry> = expected
        .iter()
        .filter(|e| e.command_class == "set" || e.command_class == "shutdown")
        .collect();

    let actual_sets: Vec<&M3_4CommandAuditEntry> = actual
        .iter()
        .filter(|e| e.command_class == "set" || e.command_class == "shutdown")
        .collect();

    // Check forbidden commands in actual
    for entry in actual {
        let cmd_upper = entry.command.to_ascii_uppercase();
        for pat in crate::constants::SMB_FORBIDDEN_PATTERNS {
            if cmd_upper.contains(pat) && entry.sent_to_transport {
                forbidden.push(format!(
                    "command='{}' matches forbidden pattern '{}'",
                    entry.command, pat
                ));
            }
        }

        // Check for semicolons (chaining)
        if entry.command.contains(';') && entry.sent_to_transport {
            forbidden.push(format!(
                "command='{}' contains semicolon (SCPI chaining)",
                entry.command
            ));
        }
    }

    // Check missing expected set commands (order-sensitive for sets)
    // We check that all expected set commands appear in actual in order
    let mut actual_idx = 0usize;
    for exp in &expected_sets {
        let mut found = false;
        while actual_idx < actual_sets.len() {
            let act = actual_sets[actual_idx];
            actual_idx += 1;
            if act.command == exp.command && act.allowed && act.sent_to_transport {
                found = true;
                break;
            }
        }
        if !found {
            missing.push(format!(
                "step={} cmd='{}' class={}",
                exp.step_id, exp.command, exp.command_class
            ));
        }
    }

    // Check unexpected actual commands (not in expected plan)
    let expected_cmds: Vec<String> = expected.iter().map(|e| e.command.clone()).collect();
    for entry in actual {
        if entry.sent_to_transport && !expected_cmds.contains(&entry.command) {
            // Allow some extra safety queries
            let is_allowed_extra = entry.command.ends_with('?')
                && (entry.command.contains("OUTP?")
                    || entry.command.contains("MOD:STAT?")
                    || entry.command.contains("SYST:ERR?")
                    || entry.command.contains("FREQ?")
                    || entry.command.contains("POW?")
                    || entry.command.contains("FM:STAT?")
                    || entry.command == "*IDN?");
            if is_allowed_extra {
                allowed_extra.push(entry.command.clone());
            } else {
                unexpected.push(format!(
                    "cmd='{}' class={} sent={}",
                    entry.command, entry.command_class, entry.sent_to_transport
                ));
            }
        }
    }

    // Check critical shutdown commands
    let shutdown_cmds = ["OUTP OFF", "MOD:STAT OFF", "FM:STAT OFF"];
    for cmd in &shutdown_cmds {
        let found = actual_sets
            .iter()
            .any(|e| e.command == *cmd && e.sent_to_transport);
        if !found {
            notes.push(format!(
                "Warning: shutdown command '{}' not found in actual audit",
                cmd
            ));
        }
    }

    // Check if check passed
    let expected_count = expected_sets.len() as u64;
    let actual_count = actual_sets.len() as u64;

    // Pass if no forbidden commands were sent and no critical missing commands
    let critical_missing: Vec<&String> = missing
        .iter()
        .filter(|m| {
            m.contains("OUTP OFF") || m.contains("MOD:STAT OFF") || m.contains("FM:STAT OFF")
        })
        .collect();

    let passed = forbidden.is_empty() && critical_missing.is_empty();

    CommandAuditComparison {
        schema_version: "0.2.0".into(),
        kind: "command_audit_comparison".into(),
        passed,
        expected_command_count: expected_count,
        actual_command_count: actual_count,
        missing_expected_commands: missing,
        unexpected_actual_commands: unexpected,
        forbidden_actual_commands: forbidden,
        allowed_extra_queries: allowed_extra,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_plan_empty_audit_passes() {
        let cmp = compare_plan_vs_audit(&[], &[]);
        assert!(cmp.passed);
        assert_eq!(cmp.expected_command_count, 0);
        assert_eq!(cmp.actual_command_count, 0);
    }

    #[test]
    fn detects_forbidden_command() {
        let expected = vec![CommandPlanEntry {
            sequence_index: 0,
            step_id: "s1".into(),
            repeat_index: 0,
            device_id: "smb100a".into(),
            command: "OUTP OFF".into(),
            command_class: "shutdown".into(),
            safety_relevant: true,
        }];

        let actual = vec![M3_4CommandAuditEntry {
            timestamp_unix_ms: 0,
            device_id: "smb100a".into(),
            command: "SWE:MODE AUTO".into(),
            command_class: "set".into(),
            allowed: false,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: None,
            transport_error: None,
            safety_relevant: true,
        }];

        let cmp = compare_plan_vs_audit(&expected, &actual);
        assert!(!cmp.forbidden_actual_commands.is_empty());
    }

    #[test]
    fn missing_outp_off_fails() {
        let expected = vec![CommandPlanEntry {
            sequence_index: 0,
            step_id: "shutdown".into(),
            repeat_index: 0,
            device_id: "smb100a".into(),
            command: "OUTP OFF".into(),
            command_class: "shutdown".into(),
            safety_relevant: true,
        }];

        let cmp = compare_plan_vs_audit(&expected, &[]);
        assert!(!cmp.passed);
        assert!(!cmp.missing_expected_commands.is_empty());
    }

    #[test]
    fn matching_commands_pass() {
        let expected = vec![CommandPlanEntry {
            sequence_index: 0,
            step_id: "shutdown".into(),
            repeat_index: 0,
            device_id: "smb100a".into(),
            command: "OUTP OFF".into(),
            command_class: "shutdown".into(),
            safety_relevant: true,
        }];

        let actual = vec![M3_4CommandAuditEntry {
            timestamp_unix_ms: 1,
            device_id: "smb100a".into(),
            command: "OUTP OFF".into(),
            command_class: "shutdown".into(),
            allowed: true,
            sent_to_transport: true,
            rejection_reason: None,
            response_preview: Some("0".into()),
            transport_error: None,
            safety_relevant: true,
        }];

        let cmp = compare_plan_vs_audit(&expected, &actual);
        assert!(cmp.passed);
    }
}

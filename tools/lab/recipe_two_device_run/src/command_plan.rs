//! Generate expected command plan from resolved recipe.

use crate::types::*;

/// Generate the full expected command plan for a recipe-shaped run.
/// This includes preflight queries, common config, per-step commands, and shutdown commands.
pub fn generate_command_plan(
    recipe: &M3_4Recipe,
    resolved: &M3_4ResolvedRecipe,
) -> (Vec<CommandPlanEntry>, CommandPlanSummary) {
    let mut entries: Vec<CommandPlanEntry> = Vec::new();
    let mut seq: u64 = 0;

    // Phase 2 — preflight queries
    let preflight_queries = &[
        "*IDN?",
        "OUTP?",
        "MOD:STAT?",
        "FREQ?",
        "POW?",
        "POW:ALC?",
        "FM:STAT?",
        "FM:SOUR?",
        "FM:DEV?",
        "SYST:ERR?",
    ];
    if recipe
        .modulation
        .internal_lf
        .as_ref()
        .is_some_and(|lf| lf.enabled)
    {
        // LF queries will be added inline below
    }
    for cmd in preflight_queries {
        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: "preflight".into(),
            repeat_index: 0,
            device_id: "smb100a".into(),
            command: cmd.to_string(),
            command_class: "query".into(),
            safety_relevant: matches!(*cmd, "OUTP?" | "MOD:STAT?" | "SYST:ERR?"),
        });
        seq += 1;
    }

    // Phase 3 — common configuration
    let config_cmds: &[(&str, String)] = &[
        ("set", format!("POW {:.1}", recipe.rf.power_dbm)),
        ("set", "POW:ALC AUTO".into()),
        ("set", "FM:SOUR INT".into()),
        (
            "set",
            format!("FM:DEV {:.0}", recipe.modulation.fm_deviation_hz),
        ),
        ("set", "FM:STAT ON".into()),
        ("set", "MOD:STAT ON".into()),
    ];
    for (cls, cmd) in config_cmds {
        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: "config".into(),
            repeat_index: 0,
            device_id: "smb100a".into(),
            command: cmd.clone(),
            command_class: cls.to_string(),
            safety_relevant: cmd == "FM:STAT ON" || cmd == "MOD:STAT ON",
        });
        seq += 1;
    }

    // LF config if enabled
    if let Some(ref lf) = recipe.modulation.internal_lf {
        if lf.enabled {
            let lf_cmds: &[(&str, String)] = &[
                ("set", format!("LFO:FREQ {:.0}", lf.frequency_hz)),
                ("set", format!("LFO:SHAP {}", lf.shape)),
                ("set", format!("LFO:VOLT {:.3}", lf.voltage_v)),
            ];
            for (cls, cmd) in lf_cmds {
                entries.push(CommandPlanEntry {
                    sequence_index: seq,
                    step_id: "config".into(),
                    repeat_index: 0,
                    device_id: "smb100a".into(),
                    command: cmd.to_string(),
                    command_class: cls.to_string(),
                    safety_relevant: false,
                });
                seq += 1;
            }
        }
    }

    // Phase 5 — per-step commands (from resolved steps)
    // We don't expand to actual frequency values here; the harness/real runner does that
    for step in &resolved.steps {
        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: step.step_id.clone(),
            repeat_index: step.repeat_index,
            device_id: "smb100a".into(),
            command: "OUTP OFF".into(),
            command_class: "set".into(),
            safety_relevant: true,
        });
        seq += 1;

        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: step.step_id.clone(),
            repeat_index: step.repeat_index,
            device_id: "smb100a".into(),
            command: format!("FREQ {:.0}", step.frequency_hz),
            command_class: "set".into(),
            safety_relevant: false,
        });
        seq += 1;

        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: step.step_id.clone(),
            repeat_index: step.repeat_index,
            device_id: "smb100a".into(),
            command: "FREQ?".into(),
            command_class: "query".into(),
            safety_relevant: false,
        });
        seq += 1;

        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: step.step_id.clone(),
            repeat_index: step.repeat_index,
            device_id: "smb100a".into(),
            command: "OUTP ON".into(),
            command_class: "set".into(),
            safety_relevant: true,
        });
        seq += 1;

        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: step.step_id.clone(),
            repeat_index: step.repeat_index,
            device_id: "smb100a".into(),
            command: "OUTP?".into(),
            command_class: "query".into(),
            safety_relevant: true,
        });
        seq += 1;

        // OE frame acquisitions
        for _f in 0..step.frames_to_acquire {
            entries.push(CommandPlanEntry {
                sequence_index: seq,
                step_id: step.step_id.clone(),
                repeat_index: step.repeat_index,
                device_id: "oe1022d".into(),
                command: "RALL?".into(),
                command_class: "oe_acquisition".into(),
                safety_relevant: false,
            });
            seq += 1;
        }

        // OUTP OFF after acquisition
        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: step.step_id.clone(),
            repeat_index: step.repeat_index,
            device_id: "smb100a".into(),
            command: "OUTP OFF".into(),
            command_class: "set".into(),
            safety_relevant: true,
        });
        seq += 1;
    }

    // Phase 6 — shutdown
    for cmd in &["OUTP OFF", "MOD:STAT OFF", "FM:STAT OFF"] {
        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: "shutdown".into(),
            repeat_index: 0,
            device_id: "smb100a".into(),
            command: cmd.to_string(),
            command_class: "shutdown".into(),
            safety_relevant: true,
        });
        seq += 1;
    }

    // Final SYST:ERR? checks
    for _ in 0..3 {
        entries.push(CommandPlanEntry {
            sequence_index: seq,
            step_id: "shutdown".into(),
            repeat_index: 0,
            device_id: "smb100a".into(),
            command: "SYST:ERR?".into(),
            command_class: "query".into(),
            safety_relevant: true,
        });
        seq += 1;
    }

    // OE identity query
    entries.push(CommandPlanEntry {
        sequence_index: seq,
        step_id: "oe_identity".into(),
        repeat_index: 0,
        device_id: "oe1022d".into(),
        command: "*IDN?".into(),
        command_class: "query".into(),
        safety_relevant: false,
    });

    let total = entries.len() as u64;
    let set_count = entries.iter().filter(|e| e.command_class == "set").count() as u64;
    let query_count = entries
        .iter()
        .filter(|e| e.command_class == "query")
        .count() as u64;
    let shutdown_count = entries
        .iter()
        .filter(|e| e.command_class == "shutdown")
        .count() as u64;
    let safety_count = entries.iter().filter(|e| e.safety_relevant).count() as u64;

    let summary = CommandPlanSummary {
        schema_version: "0.2.0".into(),
        kind: "command_plan_summary".into(),
        total_commands: total,
        set_commands: set_count,
        query_commands: query_count,
        shutdown_commands: shutdown_count,
        safety_relevant_commands: safety_count,
    };

    (entries, summary)
}

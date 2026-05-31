//! Resolved recipe generation and dry run plan for M3.4.

use crate::recipe::generate_frequencies;
use crate::types::*;

/// Build a resolved recipe from the input recipe.
pub fn build_resolved_recipe(recipe: &M3_4Recipe, recipe_hash: &str) -> M3_4ResolvedRecipe {
    let frequencies = generate_frequencies(recipe);
    let total_points = frequencies.len() as u64;
    let repeat_count = recipe.acquisition.repeat_count;
    let total_steps = total_points * repeat_count;
    let frames_per_step = recipe.acquisition.frames_per_step;

    let mut steps = Vec::with_capacity(total_steps as usize);

    for repeat in 0..repeat_count {
        for (pt_i, &freq) in frequencies.iter().enumerate() {
            let step_id = format!("repeat_{}_rf_step_{:03}", repeat, pt_i);
            let mut expected_smb_commands: Vec<String> = Vec::new();

            // OUTP OFF before freq change
            expected_smb_commands.push("OUTP OFF".into());
            // Set and verify frequency
            expected_smb_commands.push(format!("FREQ {:.0}", freq));
            expected_smb_commands.push("FREQ?".into());
            // OUTP ON for acquisition
            expected_smb_commands.push("OUTP ON".into());
            expected_smb_commands.push("OUTP?".into());

            let step_duration_ms = frames_per_step as f64
                * (800.0 + recipe.acquisition.inter_frame_delay_ms as f64)
                + 200.0; // freq change + settle

            steps.push(M3_4ResolvedStep {
                step_id,
                repeat_index: repeat,
                point_index: pt_i as u64,
                total_points,
                frequency_hz: freq,
                rf_power_dbm: recipe.rf.power_dbm,
                fm_deviation_hz: recipe.modulation.fm_deviation_hz,
                fm_on: true,
                mod_on: true,
                lf_enabled: recipe
                    .modulation
                    .internal_lf
                    .as_ref()
                    .is_some_and(|lf| lf.enabled),
                frames_to_acquire: frames_per_step,
                estimated_duration_ms: step_duration_ms,
                expected_smb_commands,
            });
        }
    }

    let estimated_duration_s = steps.iter().map(|s| s.estimated_duration_ms).sum::<f64>() / 1000.0;

    M3_4ResolvedRecipe {
        schema_version: "0.2.0".into(),
        kind: "resolved_recipe".into(),
        id: format!("resolved_{}", recipe.id),
        source_recipe_id: recipe.id.clone(),
        source_recipe_hash: recipe_hash.to_string(),
        estimated_duration_s,
        safety_report_id: None,
        total_steps,
        steps,
    }
}

/// Build a dry run plan from a resolved recipe.
pub fn build_dry_run_plan(resolved: &M3_4ResolvedRecipe) -> M3_4DryRunPlan {
    let steps: Vec<M3_4DryRunStep> = resolved
        .steps
        .iter()
        .map(|s| {
            let actions: Vec<String> = s
                .expected_smb_commands
                .iter()
                .map(|c| {
                    if c == "OUTP OFF" || c == "OUTP ON" {
                        format!("[SET] {}", c)
                    } else if c.ends_with('?') {
                        format!("[QRY] {}", c)
                    } else {
                        format!("[SET] {}", c)
                    }
                })
                .collect();
            M3_4DryRunStep {
                step_id: s.step_id.clone(),
                repeat_index: s.repeat_index,
                frequency_hz: s.frequency_hz,
                device_actions: actions,
                frames_to_acquire: s.frames_to_acquire,
                estimated_duration_ms: s.estimated_duration_ms,
            }
        })
        .collect();

    M3_4DryRunPlan {
        schema_version: "0.2.0".into(),
        kind: "dry_run_plan".into(),
        id: format!("dry_run_{}", resolved.id),
        resolved_recipe_id: resolved.id.clone(),
        summary: M3_4DryRunSummary {
            step_count: resolved.total_steps,
            total_frames: resolved.steps.iter().map(|s| s.frames_to_acquire).sum(),
            repeat_count: resolved
                .steps
                .last()
                .map(|s| s.repeat_index + 1)
                .unwrap_or(0),
            rf_points: resolved.steps.first().map(|s| s.total_points).unwrap_or(0),
            estimated_duration_s: resolved.estimated_duration_s,
            required_devices: vec!["smb100a".into(), "oe1022d".into()],
        },
        steps,
    }
}

use crate::types::{AlignmentSummary, FrameCountPerStep};

/// Build an alignment summary from per-step frame counts.
pub fn build_alignment_summary(steps_with_frame_counts: &[(u64, usize)]) -> AlignmentSummary {
    let total_frames: usize = steps_with_frame_counts.iter().map(|(_, c)| c).sum();
    let steps_with_frames = steps_with_frame_counts
        .iter()
        .filter(|(_, c)| *c > 0)
        .count();
    let frames_per_step_map: Vec<FrameCountPerStep> = steps_with_frame_counts
        .iter()
        .map(|(step_index, frame_count)| FrameCountPerStep {
            step_index: *step_index,
            frame_count: *frame_count,
        })
        .collect();

    let alignment_ok = steps_with_frame_counts.iter().all(|(_, c)| *c > 0);

    AlignmentSummary {
        schema_version: "0.2.0".into(),
        total_frames,
        steps_with_frames,
        frames_per_step_map,
        alignment_ok,
    }
}

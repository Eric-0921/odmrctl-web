use crate::types::{AlignmentSummary, FrameCountPerStep};

/// Build an alignment summary from per-step (repeat_index, step_index, frame_count) tuples.
pub fn build_alignment_summary(
    steps_with_frame_counts: &[(u64, u64, usize)],
) -> AlignmentSummary {
    let total_frames: usize = steps_with_frame_counts.iter().map(|(_, _, c)| c).sum();
    let steps_with_frames = steps_with_frame_counts
        .iter()
        .filter(|(_, _, c)| *c > 0)
        .count();
    let frames_per_step_map: Vec<FrameCountPerStep> = steps_with_frame_counts
        .iter()
        .map(|(repeat_index, step_index, frame_count)| FrameCountPerStep {
            repeat_index: *repeat_index,
            step_index: *step_index,
            frame_count: *frame_count,
        })
        .collect();

    let alignment_ok = steps_with_frame_counts.iter().all(|(_, _, c)| *c > 0);

    AlignmentSummary {
        schema_version: "0.2.0".into(),
        total_frames,
        steps_with_frames,
        frames_per_step_map,
        alignment_ok,
    }
}

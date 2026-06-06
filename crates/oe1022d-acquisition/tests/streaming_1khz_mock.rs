//! Streaming integration test: end-to-end mock 1 kHz run for
//! 1 second. Verifies the parser + writer pipeline can keep up
//! with the full 1000 sample/s data rate (vs. RALL? at ~62.5
//! sample/s).
//!
//! C13 scope. The real-device path is C15.

use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use oe1022d_acquisition::{RunConfig, RunWriter};
use oe1022d_transport::{
    spawn_mock_streaming_reader, MockStreamingLink, StreamingConfig, StreamingSample,
};
use parking_lot::Mutex;

#[test]
fn mock_1khz_one_second_yields_about_1000_samples() {
    let dir = std::env::temp_dir().join(format!(
        "oe1022d_1khz_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    // 1 ms step = 1 kHz. Run for 1 s. The mock produces 100
    // samples per read_floats call; with a 0 ms poll_delay
    // we'll likely overshoot the 1000 mark by a little, which
    // is fine.
    let config = StreamingConfig {
        step_time_ms: 1,
        sample_length: 16384,
        run_mode: 1,
        trigger_mode: 0,
        buffer_index: 1,
        buffer_parameter: 4,
    };
    let link = MockStreamingLink::new(config.clone());
    let (tx, rx) = unbounded::<Vec<StreamingSample>>();
    let _handle = spawn_mock_streaming_reader(
        link,
        config,
        "SSI:LIA-OE1022D:STREAM_MOCK".to_string(),
        tx,
        Duration::from_millis(0), // poll as fast as possible
    );

    let writer = RunWriter::create(RunConfig {
        run_dir: dir.clone(),
        run_id: "1khz_1s".to_string(),
        fields: vec![], // streaming path writes StreamingSample directly
        buffer_bytes: 16 * 1024,
    })
    .expect("create writer");

    let started = Instant::now();
    std::thread::sleep(Duration::from_millis(1000));

    let mut total_samples: u64 = 0;
    let mut total_batches: u64 = 0;
    while let Ok(batch) = rx.try_recv() {
        total_batches += 1;
        for s in &batch {
            let _ = writer.write_sample(&oe1022d_transport::ParsedSample {
                // Reuse ParsedSample's shape so the writer is
                // happy. We synthesize the ParsedSample from a
                // StreamingSample so the on-disk format is the
                // same as C7's RALL? path.
                sample_in_frame: 0,
                t_mono_ns: s.t_mono_ns,
                t_wall_ns: s.t_wall_ns,
                t_wall_ms: s.t_wall_ms,
                device_id: s.device_id.clone(),
                frame_sequence_no: s.stream_sequence,
                field: match s.field.as_str() {
                    "BX" => oe1022d_transport::SampleField::BX,
                    "BY" => oe1022d_transport::SampleField::BY,
                    "BFreq" => oe1022d_transport::SampleField::BFreq,
                    _ => oe1022d_transport::SampleField::BX,
                },
                value: s.value,
                status: oe1022d_transport::SampleStatus {
                    transport_ok: true,
                    frame_was_exact_size: true,
                },
                partial_warmup: false,
            });
            total_samples += 1;
        }
    }
    writer.flush().expect("flush");
    let elapsed = started.elapsed();

    eprintln!(
        "1khz_1s: {} samples in {} batches over {:?}",
        total_samples, total_batches, elapsed
    );

    // The mock stream is unbounded and we let it run for ~1 s.
    // We should see at least 800 samples (allowing for the
    // race between sleep and reader thread startup). The
    // upper bound is much higher because the mock is CPU-bound.
    assert!(
        total_samples >= 800,
        "expected >= 800 samples in 1s, got {total_samples}"
    );
    let stats = writer.stats();
    assert_eq!(stats.samples_written, total_samples);

    // Validate ndjson file shape.
    let content = std::fs::read_to_string(dir.join("samples.ndjson")).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len() as u64, total_samples);
    for line in lines.iter().take(3) {
        let v: serde_json::Value = serde_json::from_str(line)
            .expect("each ndjson line must be valid JSON");
        assert!(v.get("device_id").is_some(), "missing device_id");
        assert!(v.get("t_mono_ns").is_some(), "missing t_mono_ns");
        assert!(v.get("value").is_some(), "missing value");
    }
    // Sequence numbers must be contiguous (1 per sample).
    let mut last_seq: u64 = 0;
    let mut saw_first = false;
    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        let seq = v["frame_sequence_no"].as_u64().unwrap();
        if !saw_first {
            assert_eq!(seq, 0);
            saw_first = true;
        } else if seq != last_seq {
            // Per-sample stream_sequence must be contiguous.
            assert_eq!(seq, last_seq + 1, "stream_sequence must be contiguous");
        }
        last_seq = seq;
    }

    writer.shutdown().expect("shutdown");
    let _ = std::fs::remove_dir_all(&dir);
}

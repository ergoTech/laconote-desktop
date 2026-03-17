use opus::{Application, Bitrate, Channels, Encoder as OpusEncoder};
use tracing::info;

pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME_SAMPLES: usize = 960; // 20 ms at 48 kHz
const OPUS_BITRATE: i32 = 128_000;
const FRAMES_PER_CLUSTER: usize = 1_500; // 30 s per cluster (1500 × 20 ms)

/// Encode raw f32 PCM (48 kHz, mono) to a complete, self-contained WebM/Opus file.
///
/// Returns valid WebM bytes beginning with the EBML magic `0x1A 0x45 0xDF 0xA3`.
pub fn encode_to_webm(pcm: &[f32]) -> Result<Vec<u8>, String> {
    let mut encoder = OpusEncoder::new(SAMPLE_RATE, Channels::Mono, Application::Voip)
        .map_err(|e| format!("Opus encoder init failed: {e}"))?;
    encoder
        .set_bitrate(Bitrate::Bits(OPUS_BITRATE))
        .map_err(|e| format!("Opus set_bitrate failed: {e}"))?;

    let mut out_buf = [0u8; 4096];
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut pos = 0usize;

    while pos + FRAME_SAMPLES <= pcm.len() {
        let n = encoder
            .encode_float(&pcm[pos..pos + FRAME_SAMPLES], &mut out_buf)
            .map_err(|e| format!("Opus encode failed at sample {pos}: {e}"))?;
        frames.push(out_buf[..n].to_vec());
        pos += FRAME_SAMPLES;
    }

    if pos < pcm.len() {
        let mut padded = pcm[pos..].to_vec();
        padded.resize(FRAME_SAMPLES, 0.0);
        let n = encoder
            .encode_float(&padded, &mut out_buf)
            .map_err(|e| format!("Opus encode (pad) failed: {e}"))?;
        frames.push(out_buf[..n].to_vec());
    }

    info!(
        input_samples = pcm.len(),
        opus_frames = frames.len(),
        "Encoded PCM → Opus frames"
    );

    Ok(build_webm(&frames))
}

// ── Minimal EBML / WebM muxer ────────────────────────────────────────────────

/// Encode n as an EBML VINT (variable-length integer for data sizes).
fn vint(n: u64) -> Vec<u8> {
    if n < 0x7F {
        vec![(n | 0x80) as u8]
    } else if n < 0x3FFF {
        vec![((n >> 8) | 0x40) as u8, (n & 0xFF) as u8]
    } else if n < 0x001F_FFFF {
        vec![
            ((n >> 16) | 0x20) as u8,
            ((n >> 8) & 0xFF) as u8,
            (n & 0xFF) as u8,
        ]
    } else if n < 0x0FFF_FFFF {
        vec![
            ((n >> 24) | 0x10) as u8,
            ((n >> 16) & 0xFF) as u8,
            ((n >> 8) & 0xFF) as u8,
            (n & 0xFF) as u8,
        ]
    } else {
        vec![
            0x01,
            ((n >> 48) & 0xFF) as u8,
            ((n >> 40) & 0xFF) as u8,
            ((n >> 32) & 0xFF) as u8,
            ((n >> 24) & 0xFF) as u8,
            ((n >> 16) & 0xFF) as u8,
            ((n >> 8) & 0xFF) as u8,
            (n & 0xFF) as u8,
        ]
    }
}

/// Write an EBML element: ID bytes + VINT size + data.
fn elem(id: &[u8], data: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend(vint(data.len() as u64));
    v.extend_from_slice(data);
    v
}

/// Encode an unsigned integer as big-endian bytes of the given width.
fn uint_be(n: u64, width: usize) -> Vec<u8> {
    (0..width)
        .rev()
        .map(|i| ((n >> (i * 8)) & 0xFF) as u8)
        .collect()
}

/// Encode a f64 as big-endian IEEE 754 bytes.
fn f64_be(f: f64) -> Vec<u8> {
    f.to_be_bytes().to_vec()
}

fn build_webm(frames: &[Vec<u8>]) -> Vec<u8> {
    // ── OpusHead (CodecPrivate) ──────────────────────────────────────────────
    let opus_head = {
        let mut h = Vec::new();
        h.extend_from_slice(b"OpusHead");
        h.push(1); // version
        h.push(1); // channels (mono)
        h.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
        h.extend_from_slice(&(SAMPLE_RATE as u32).to_le_bytes()); // input sample rate
        h.extend_from_slice(&0u16.to_le_bytes()); // output gain
        h.push(0); // channel mapping family (RTP)
        h
    };

    // ── Audio sub-element ────────────────────────────────────────────────────
    let audio = {
        let mut a = Vec::new();
        a.extend(elem(&[0xB5], &f64_be(SAMPLE_RATE as f64))); // SamplingFrequency
        a.extend(elem(&[0x9F], &uint_be(1, 1))); // Channels
        elem(&[0xE1], &a)
    };

    // ── TrackEntry ───────────────────────────────────────────────────────────
    let track_entry = {
        let mut t = Vec::new();
        t.extend(elem(&[0xD7], &uint_be(1, 1))); // TrackNumber
        t.extend(elem(&[0x73, 0xC5], &uint_be(1, 8))); // TrackUID
        t.extend(elem(&[0x83], &uint_be(2, 1))); // TrackType = audio (2)
        t.extend(elem(&[0x86], b"A_OPUS")); // CodecID
        t.extend(elem(&[0x63, 0xA2], &opus_head)); // CodecPrivate
        t.extend(audio);
        elem(&[0xAE], &t)
    };

    let tracks = elem(&[0x16, 0x54, 0xAE, 0x6B], &track_entry);

    // ── Info ─────────────────────────────────────────────────────────────────
    let info = {
        let mut i = Vec::new();
        i.extend(elem(&[0x2A, 0xD7, 0xB1], &uint_be(1_000_000, 4))); // TimestampScale = 1 ms
        i.extend(elem(&[0x4D, 0x80], b"laconote-desktop")); // MuxingApp
        i.extend(elem(&[0x57, 0x41], b"laconote-desktop")); // WritingApp
        elem(&[0x15, 0x49, 0xA9, 0x66], &i)
    };

    // ── Clusters (≤ FRAMES_PER_CLUSTER frames each) ──────────────────────────
    let mut clusters_bytes: Vec<u8> = Vec::new();
    for (chunk_idx, chunk) in frames.chunks(FRAMES_PER_CLUSTER).enumerate() {
        let cluster_ts_ms = (chunk_idx * FRAMES_PER_CLUSTER * 20) as u64;

        let mut c = Vec::new();
        c.extend(elem(&[0xE7], &uint_be(cluster_ts_ms, 4))); // Timestamp

        for (frame_idx, frame) in chunk.iter().enumerate() {
            let relative_ms = (frame_idx * 20) as i16;
            let mut sb = Vec::new();
            sb.push(0x81u8); // TrackNumber VINT (track 1)
            sb.extend_from_slice(&relative_ms.to_be_bytes()); // relative timecode
            sb.push(0x80); // flags: keyframe
            sb.extend_from_slice(frame);
            c.extend(elem(&[0xA3], &sb)); // SimpleBlock
        }

        clusters_bytes.extend(elem(&[0x1F, 0x43, 0xB6, 0x75], &c));
    }

    // ── Segment ──────────────────────────────────────────────────────────────
    let mut segment_body = Vec::new();
    segment_body.extend(info);
    segment_body.extend(tracks);
    segment_body.extend(clusters_bytes);
    let segment = elem(&[0x18, 0x53, 0x80, 0x67], &segment_body);

    // ── EBML Header ──────────────────────────────────────────────────────────
    let ebml_header = {
        let mut h = Vec::new();
        h.extend(elem(&[0x42, 0x86], &uint_be(1, 1))); // EBMLVersion
        h.extend(elem(&[0x42, 0xF7], &uint_be(1, 1))); // EBMLReadVersion
        h.extend(elem(&[0x42, 0xF2], &uint_be(4, 1))); // EBMLMaxIDLength
        h.extend(elem(&[0x42, 0xF3], &uint_be(8, 1))); // EBMLMaxSizeLength
        h.extend(elem(&[0x42, 0x82], b"webm")); // DocType
        h.extend(elem(&[0x42, 0x87], &uint_be(4, 1))); // DocTypeVersion
        h.extend(elem(&[0x42, 0x85], &uint_be(2, 1))); // DocTypeReadVersion
        elem(&[0x1A, 0x45, 0xDF, 0xA3], &h)
    };

    let mut out = ebml_header;
    out.extend(segment);
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EBML_MAGIC: &[u8] = &[0x1A, 0x45, 0xDF, 0xA3];

    fn sine_wave(freq_hz: f32, duration_secs: f32) -> Vec<f32> {
        let samples = (SAMPLE_RATE as f32 * duration_secs) as usize;
        (0..samples)
            .map(|i| {
                (2.0 * std::f32::consts::PI * freq_hz * i as f32 / SAMPLE_RATE as f32).sin()
                    * 0.5
            })
            .collect()
    }

    #[test]
    fn test_encode_produces_valid_webm_magic() {
        let pcm = sine_wave(440.0, 1.0);
        let webm = encode_to_webm(&pcm).expect("encode_to_webm failed");
        assert!(
            webm.starts_with(EBML_MAGIC),
            "Output should start with EBML magic bytes"
        );
        assert!(webm.len() > 256, "WebM output too small: {} bytes", webm.len());
    }

    #[test]
    fn test_encode_empty_pcm_produces_header_only() {
        let webm = encode_to_webm(&[]).expect("encode_to_webm with empty input failed");
        assert!(
            webm.starts_with(EBML_MAGIC),
            "Even empty WebM should start with EBML magic"
        );
    }

    #[test]
    fn test_encode_long_audio_produces_multiple_clusters() {
        let pcm = sine_wave(440.0, 31.0); // 31 seconds → 2 clusters
        let webm = encode_to_webm(&pcm).expect("encode failed");
        assert!(webm.starts_with(EBML_MAGIC));
        let cluster_id: &[u8] = &[0x1F, 0x43, 0xB6, 0x75];
        let occurrences = webm
            .windows(4)
            .filter(|w| *w == cluster_id)
            .count();
        assert!(
            occurrences >= 2,
            "31 seconds of audio should produce ≥ 2 clusters, got {occurrences}"
        );
    }

    #[test]
    fn test_vint_encodes_small_value_in_one_byte() {
        assert_eq!(vint(0), vec![0x80]);
        assert_eq!(vint(1), vec![0x81]);
        assert_eq!(vint(126), vec![0xFE]);
    }

    #[test]
    fn test_vint_encodes_boundary_value_in_two_bytes() {
        let v = vint(127);
        assert_eq!(v.len(), 2, "127 should require 2 bytes");
        assert_eq!(v[0] & 0xC0, 0x40, "First byte should have 2-byte marker");
    }

    #[test]
    fn test_vint_encodes_large_value_correctly() {
        let v = vint(16382);
        assert_eq!(v.len(), 2);
    }
}

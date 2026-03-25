pub fn mix_to_mono(data: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }
    let frames = data.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        let sum: f32 = (0..channels).map(|ch| data[frame * channels + ch]).sum();
        mono.push(sum / channels as f32);
    }
    mono
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_passthrough() {
        let input = vec![0.1, 0.2, 0.3, 0.4];
        let result = mix_to_mono(&input, 1);
        assert_eq!(result, input);
    }

    #[test]
    fn mono_passthrough_zero_channels() {
        let input = vec![0.5, 0.6];
        let result = mix_to_mono(&input, 0);
        assert_eq!(result, input);
    }

    #[test]
    fn stereo_downmix() {
        let input = vec![1.0, 0.0, 0.0, 1.0];
        let result = mix_to_mono(&input, 2);
        assert_eq!(result, vec![0.5, 0.5]);
    }

    #[test]
    fn multi_channel_downmix() {
        let input = vec![1.0, 0.0, 0.5, 0.0, 0.0, 0.0];
        let result = mix_to_mono(&input, 3);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.5).abs() < f32::EPSILON);
        assert_eq!(result[1], 0.0);
    }

    #[test]
    fn empty_input() {
        let result = mix_to_mono(&[], 2);
        assert!(result.is_empty());
    }
}

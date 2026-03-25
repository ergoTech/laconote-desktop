use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

const SAMPLE_RATE: u32 = 48_000;

pub struct ShadowBuffer {
    inner: Mutex<RingInner>,
    capacity_samples: usize,
    total_written: AtomicU64,
}

struct RingInner {
    data: Vec<f32>,
    write_pos: usize,
    count: usize,
}

impl ShadowBuffer {
    pub fn new(buffer_minutes: u32) -> Self {
        let capacity_samples = buffer_minutes as usize * SAMPLE_RATE as usize * 60;
        Self {
            inner: Mutex::new(RingInner {
                data: vec![0.0; capacity_samples],
                write_pos: 0,
                count: 0,
            }),
            capacity_samples,
            total_written: AtomicU64::new(0),
        }
    }

    pub fn write(&self, samples: &[f32]) {
        let mut inner = self.inner.lock().unwrap();
        let cap = self.capacity_samples;
        if cap == 0 || samples.is_empty() {
            return;
        }

        let src = if samples.len() > cap {
            &samples[samples.len() - cap..]
        } else {
            samples
        };

        let pos = inner.write_pos;
        let first_chunk = cap - pos;

        if src.len() <= first_chunk {
            inner.data[pos..pos + src.len()].copy_from_slice(src);
            inner.write_pos = (pos + src.len()) % cap;
        } else {
            inner.data[pos..pos + first_chunk].copy_from_slice(&src[..first_chunk]);
            let remaining = src.len() - first_chunk;
            inner.data[..remaining].copy_from_slice(&src[first_chunk..]);
            inner.write_pos = remaining;
        }

        inner.count = (inner.count + src.len()).min(cap);

        self.total_written
            .fetch_add(samples.len() as u64, Ordering::Relaxed);
    }

    pub fn snapshot_and_clear(&self) -> Vec<f32> {
        let mut inner = self.inner.lock().unwrap();
        let result = self.read_ordered(&inner);
        inner.write_pos = 0;
        inner.count = 0;
        result
    }

    pub fn drain(&self) -> Vec<f32> {
        let inner = self.inner.lock().unwrap();
        self.read_ordered(&inner)
    }

    pub fn duration_ms(&self) -> u64 {
        let inner = self.inner.lock().unwrap();
        (inner.count as u64 * 1000) / SAMPLE_RATE as u64
    }

    pub fn capacity_ms(&self) -> u64 {
        (self.capacity_samples as u64 * 1000) / SAMPLE_RATE as u64
    }

    pub fn fill_percent(&self) -> u8 {
        if self.capacity_samples == 0 {
            return 0;
        }
        let inner = self.inner.lock().unwrap();
        ((inner.count as u64 * 100) / self.capacity_samples as u64) as u8
    }

    pub fn total_written_duration_ms(&self) -> u64 {
        let total = self.total_written.load(Ordering::Relaxed);
        (total * 1000) / SAMPLE_RATE as u64
    }

    fn read_ordered(&self, inner: &RingInner) -> Vec<f32> {
        if inner.count == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(inner.count);
        if inner.count < self.capacity_samples {
            result.extend_from_slice(&inner.data[..inner.count]);
        } else {
            result.extend_from_slice(&inner.data[inner.write_pos..]);
            result.extend_from_slice(&inner.data[..inner.write_pos]);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf = ShadowBuffer::new(1);
        assert_eq!(buf.duration_ms(), 0);
        assert_eq!(buf.fill_percent(), 0);
        assert_eq!(buf.total_written_duration_ms(), 0);
        assert!(buf.drain().is_empty());
    }

    #[test]
    fn test_capacity_ms() {
        let buf = ShadowBuffer::new(20);
        assert_eq!(buf.capacity_ms(), 20 * 60 * 1000);
    }

    #[test]
    fn test_write_and_drain() {
        let buf = ShadowBuffer::new(1);
        let samples: Vec<f32> = (0..48_000).map(|i| i as f32 / 48_000.0).collect();
        buf.write(&samples);

        assert_eq!(buf.duration_ms(), 1000);
        let drained = buf.drain();
        assert_eq!(drained.len(), 48_000);
        assert_eq!(drained[0], 0.0);
        assert!((drained[47_999] - 47_999.0 / 48_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_write_preserves_order_no_wrap() {
        let buf = ShadowBuffer::new(1);
        let samples: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        buf.write(&samples);

        let drained = buf.drain();
        assert_eq!(drained, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_overflow_drops_oldest() {
        let buf = ShadowBuffer::new(1);
        let cap = 60 * SAMPLE_RATE as usize;

        let first_batch: Vec<f32> = vec![1.0; cap];
        buf.write(&first_batch);
        assert_eq!(buf.fill_percent(), 100);

        let overflow: Vec<f32> = vec![2.0; SAMPLE_RATE as usize];
        buf.write(&overflow);

        let drained = buf.drain();
        assert_eq!(drained.len(), cap);

        assert_eq!(drained[0], 1.0);
        let last_sec = &drained[cap - SAMPLE_RATE as usize..];
        for &s in last_sec {
            assert_eq!(s, 2.0);
        }
    }

    #[test]
    fn test_snapshot_and_clear_resets_buffer() {
        let buf = ShadowBuffer::new(1);
        buf.write(&[1.0, 2.0, 3.0]);

        let snapshot = buf.snapshot_and_clear();
        assert_eq!(snapshot, vec![1.0, 2.0, 3.0]);

        assert_eq!(buf.duration_ms(), 0);
        assert_eq!(buf.fill_percent(), 0);
        assert!(buf.drain().is_empty());
    }

    #[test]
    fn test_snapshot_preserves_total_written() {
        let buf = ShadowBuffer::new(1);
        buf.write(&[1.0; 48_000]);
        let _ = buf.snapshot_and_clear();

        assert_eq!(buf.total_written_duration_ms(), 1000);

        buf.write(&[2.0; 48_000]);
        assert_eq!(buf.total_written_duration_ms(), 2000);
    }

    #[test]
    fn test_wrapped_buffer_order() {
        let buf = ShadowBuffer::new(1);
        let cap = 60 * SAMPLE_RATE as usize;

        let first: Vec<f32> = (0..cap).map(|i| i as f32).collect();
        buf.write(&first);

        let extra: Vec<f32> = vec![999.0; 100];
        buf.write(&extra);

        let drained = buf.drain();
        assert_eq!(drained.len(), cap);
        assert_eq!(drained[0], 100.0);
        for i in 0..100 {
            assert_eq!(drained[cap - 100 + i], 999.0);
        }
    }

    #[test]
    fn test_fill_percent_gradual() {
        let buf = ShadowBuffer::new(1);
        let cap = 60 * SAMPLE_RATE as usize;

        let half: Vec<f32> = vec![0.0; cap / 2];
        buf.write(&half);
        assert_eq!(buf.fill_percent(), 50);

        let quarter: Vec<f32> = vec![0.0; cap / 4];
        buf.write(&quarter);
        assert_eq!(buf.fill_percent(), 75);
    }

    #[test]
    fn test_multiple_small_writes() {
        let buf = ShadowBuffer::new(1);
        for i in 0..100 {
            buf.write(&[i as f32]);
        }
        let drained = buf.drain();
        assert_eq!(drained.len(), 100);
        for i in 0..100 {
            assert_eq!(drained[i], i as f32);
        }
    }

    #[test]
    fn test_write_after_snapshot_and_clear() {
        let buf = ShadowBuffer::new(1);
        buf.write(&[1.0, 2.0, 3.0]);
        let _ = buf.snapshot_and_clear();

        buf.write(&[4.0, 5.0]);
        let drained = buf.drain();
        assert_eq!(drained, vec![4.0, 5.0]);
    }

    #[test]
    fn test_zero_capacity_buffer() {
        let buf = ShadowBuffer::new(0);
        buf.write(&[1.0, 2.0]);
        assert_eq!(buf.duration_ms(), 0);
        assert_eq!(buf.fill_percent(), 0);
        assert!(buf.drain().is_empty());
    }
}

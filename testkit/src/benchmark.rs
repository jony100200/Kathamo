//! Frame-time telemetry: percentiles (p50, p95, p99), frame budget (16.7ms), and stall detection.

#[derive(Debug, Clone, PartialEq)]
pub struct FrameMetrics {
    pub total_frames: usize,
    pub p50_ms: f32,
    pub p90_ms: f32,
    pub p95_ms: f32,
    pub p99_ms: f32,
    pub max_ms: f32,
    pub stalls_over_16ms: usize,
    pub stalls_over_33ms: usize,
}

pub struct BenchmarkRecorder {
    frame_durations_us: Vec<u64>,
}

impl Default for BenchmarkRecorder {
    fn default() -> Self {
        Self {
            frame_durations_us: Vec::with_capacity(1000),
        }
    }
}

impl BenchmarkRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_frame_us(&mut self, us: u64) {
        self.frame_durations_us.push(us);
    }

    pub fn clear(&mut self) {
        self.frame_durations_us.clear();
    }

    pub fn compute_metrics(&self) -> FrameMetrics {
        if self.frame_durations_us.is_empty() {
            return FrameMetrics {
                total_frames: 0,
                p50_ms: 0.0,
                p90_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                max_ms: 0.0,
                stalls_over_16ms: 0,
                stalls_over_33ms: 0,
            };
        }

        let mut sorted = self.frame_durations_us.clone();
        sorted.sort_unstable();

        let count = sorted.len();
        let percentile = |pct: f32| -> f32 {
            let idx = ((pct / 100.0) * (count as f32)).clamp(0.0, (count - 1) as f32) as usize;
            sorted[idx] as f32 / 1000.0
        };

        let mut stalls_16 = 0;
        let mut stalls_33 = 0;

        for &us in &self.frame_durations_us {
            let ms = us as f32 / 1000.0;
            if ms > 33.3 {
                stalls_33 += 1;
            } else if ms > 16.7 {
                stalls_16 += 1;
            }
        }

        FrameMetrics {
            total_frames: count,
            p50_ms: percentile(50.0),
            p90_ms: percentile(90.0),
            p95_ms: percentile(95.0),
            p99_ms: percentile(99.0),
            max_ms: *sorted.last().unwrap() as f32 / 1000.0,
            stalls_over_16ms: stalls_16,
            stalls_over_33ms: stalls_33,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_recorder_computes_accurate_percentiles() {
        let mut recorder = BenchmarkRecorder::new();
        // Record 100 frames at ~8ms (120fps) and 5 frames at 18ms
        for _ in 0..95 {
            recorder.record_frame_us(8_000);
        }
        for _ in 0..5 {
            recorder.record_frame_us(18_000);
        }

        let metrics = recorder.compute_metrics();
        assert_eq!(metrics.total_frames, 100);
        assert!(metrics.p50_ms <= 8.5);
        assert!(metrics.p95_ms <= 18.0);
        assert_eq!(metrics.stalls_over_16ms, 5);
        assert_eq!(metrics.stalls_over_33ms, 0);
    }
}

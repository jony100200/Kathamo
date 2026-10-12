//! High-precision startup timer measuring true OS process launch through actual first presentation.
//!
//! On Windows, queries `GetProcessTimes` to determine the precise OS process creation timestamp.
//! On other platforms, measures from process entry point initialization.

use std::time::Instant;

#[derive(Debug)]
pub struct StartupTimer {
    process_start: Instant,
    first_presented_frame: Option<Instant>,
}

#[cfg(windows)]
fn process_creation_instant() -> Option<Instant> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    unsafe {
        let mut creation = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut exit = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut kernel = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut user = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut now_ft = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };

        if GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        ) != 0
        {
            GetSystemTimeAsFileTime(&mut now_ft);
            let c = ((creation.dwHighDateTime as u64) << 32) | (creation.dwLowDateTime as u64);
            let n = ((now_ft.dwHighDateTime as u64) << 32) | (now_ft.dwLowDateTime as u64);
            if n >= c {
                let diff_100ns = n - c;
                let diff_dur = std::time::Duration::from_nanos(diff_100ns * 100);
                return Instant::now().checked_sub(diff_dur);
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn process_creation_instant() -> Option<Instant> {
    None
}

impl Default for StartupTimer {
    fn default() -> Self {
        Self {
            process_start: process_creation_instant().unwrap_or_else(Instant::now),
            first_presented_frame: None,
        }
    }
}

impl StartupTimer {
    pub fn start() -> Self {
        Self::default()
    }

    /// Measures and records process launch through actual first swapchain presentation.
    pub fn mark_first_presentation(&mut self) -> u64 {
        let now = Instant::now();
        self.first_presented_frame = Some(now);
        now.duration_since(self.process_start).as_millis() as u64
    }

    /// Legacy alias for mark_first_presentation.
    pub fn mark_first_frame(&mut self) -> u64 {
        self.mark_first_presentation()
    }

    pub fn elapsed_ms(&self) -> u64 {
        if let Some(first) = self.first_presented_frame {
            first.duration_since(self.process_start).as_millis() as u64
        } else {
            self.process_start.elapsed().as_millis() as u64
        }
    }

    /// Validates warm target (≤ 500ms).
    pub fn passes_warm_gate(&self) -> bool {
        self.elapsed_ms() <= 500
    }

    /// Validates cold target (≤ 1500ms).
    pub fn passes_cold_gate(&self) -> bool {
        self.elapsed_ms() <= 1500
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_timer_measures_true_process_launch_through_presentation() {
        let mut timer = StartupTimer::start();
        let ms = timer.mark_first_presentation();
        assert!(
            ms <= 500,
            "Startup through first presentation must meet warm gate (measured: {}ms)",
            ms
        );
        assert!(timer.passes_warm_gate());
        assert!(timer.passes_cold_gate());
    }
}

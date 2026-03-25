use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(10);

pub struct HealthMonitor {
    running: Arc<AtomicBool>,
}

impl HealthMonitor {
    pub fn start(
        mixer_running: Arc<AtomicBool>,
        catap_running: Arc<AtomicBool>,
        mic_running: Arc<AtomicBool>,
        warning_tx: Sender<String>,
    ) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_thread = Arc::clone(&running);

        thread::Builder::new()
            .name("laconote-health-monitor".into())
            .spawn(move || {
                info!("Health monitor started");
                while running_thread.load(Ordering::Relaxed) {
                    thread::sleep(HEALTH_CHECK_INTERVAL);
                    if !running_thread.load(Ordering::Relaxed) {
                        break;
                    }

                    if !mixer_running.load(Ordering::Relaxed) {
                        warn!("Health check: mixer thread stopped unexpectedly");
                        let _ = warning_tx.try_send(
                            "Audio mixer stopped unexpectedly".into(),
                        );
                    }

                    if !catap_running.load(Ordering::Relaxed) {
                        warn!("Health check: CATap capture stopped unexpectedly");
                        let _ = warning_tx.try_send(
                            "System audio capture stopped unexpectedly".into(),
                        );
                    }

                    if !mic_running.load(Ordering::Relaxed) {
                        warn!("Health check: microphone capture stopped unexpectedly");
                        let _ = warning_tx.try_send(
                            "Microphone capture stopped unexpectedly".into(),
                        );
                    }
                }
                info!("Health monitor stopped");
            })
            .expect("Failed to spawn health monitor thread");

        HealthMonitor { running }
    }

    pub fn stop(self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;

    #[test]
    fn test_health_monitor_starts_and_stops() {
        let mixer = Arc::new(AtomicBool::new(true));
        let catap = Arc::new(AtomicBool::new(true));
        let mic = Arc::new(AtomicBool::new(true));
        let (tx, _rx) = bounded(32);

        let monitor = HealthMonitor::start(
            Arc::clone(&mixer),
            Arc::clone(&catap),
            Arc::clone(&mic),
            tx,
        );
        assert!(monitor.running.load(Ordering::Relaxed));
        monitor.stop();
    }

    #[test]
    fn test_health_monitor_detects_dead_component() {
        let mixer = Arc::new(AtomicBool::new(true));
        let catap = Arc::new(AtomicBool::new(false));
        let mic = Arc::new(AtomicBool::new(true));
        let (tx, rx) = bounded(32);

        let monitor = HealthMonitor::start(
            Arc::clone(&mixer),
            Arc::clone(&catap),
            Arc::clone(&mic),
            tx,
        );

        thread::sleep(Duration::from_secs(11));

        let mut warnings = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            warnings.push(msg);
        }

        monitor.stop();

        assert!(
            warnings.iter().any(|w| w.contains("System audio")),
            "Expected a warning about system audio capture, got: {warnings:?}"
        );
    }
}

use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};

pub struct AdaptiveRateLimiter {
    request_history: Mutex<VecDeque<Instant>>,
    time_window: Duration,
    min_delay_ms: u64,
    burst_threshold: usize,
}

impl AdaptiveRateLimiter {
    pub fn new(time_window_secs: u64, min_delay_ms: u64, burst_threshold: usize) -> Self {
        Self {
            request_history: Mutex::new(VecDeque::new()),
            time_window: Duration::from_secs(time_window_secs),
            min_delay_ms,
            burst_threshold,
        }
    }

    pub async fn wait(&self) {
        let mut history = self.request_history.lock().await;
        let now = Instant::now();

        // Clean up history
        while let Some(&oldest) = history.front() {
            if now.duration_since(oldest) > self.time_window {
                history.pop_front();
            } else {
                break;
            }
        }

        let recent_requests = history.len();

        // Burst protection
        if recent_requests >= self.burst_threshold {
            if let Some(&oldest) = history.front() {
                let time_until_oldest_expires = self.time_window
                    .saturating_sub(now.duration_since(oldest));

                if time_until_oldest_expires > Duration::from_millis(0) {
                    let wait_time = time_until_oldest_expires + Duration::from_millis(500);
                    log::warn!(
                        "BURST LIMIT: Waiting {}ms ({}/{} requests)",
                        wait_time.as_millis(),
                        recent_requests,
                        self.burst_threshold
                    );
                    sleep(wait_time).await;

                    // Re-clean
                    let now = Instant::now();
                    while let Some(&oldest) = history.front() {
                        if now.duration_since(oldest) > self.time_window {
                            history.pop_front();
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Calculate adaptive delay
        let base_delay = self.calculate_delay(history.len());
        let guaranteed_delay = base_delay.max(self.min_delay_ms);

        let jitter = fastrand::i64(-25..=25);
        let final_delay_ms = ((guaranteed_delay as i64) * (100 + jitter) / 100)
            .max(self.min_delay_ms as i64) as u64;

        let level = self.get_load_level(history.len());

        log::debug!(
            "Rate limiter [{}]: {}ms delay ({}/{} requests)",
            level,
            final_delay_ms,
            history.len(),
            self.burst_threshold
        );

        sleep(Duration::from_millis(final_delay_ms)).await;

        history.push_back(Instant::now());

        while history.len() > self.burst_threshold + 5 {
            history.pop_front();
        }
    }

    fn calculate_delay(&self, recent_requests: usize) -> u64 {
        match recent_requests {
            0 => 800,
            1 => 1000,
            2 => 1200,
            3 => 1500,
            4 => 2000,
            5 => 2500,
            6 => 3000,
            7 => 3500,
            8 => 4000,
            9 => 5000,
            _ => 6000,
        }
    }

    fn get_load_level(&self, recent_requests: usize) -> &'static str {
        let percent = (recent_requests * 100) / self.burst_threshold;
        match percent {
            0..=20 => "IDLE",
            21..=40 => "LOW",
            41..=60 => "MEDIUM",
            61..=80 => "HIGH",
            _ => "CRITICAL",
        }
    }
}

// ===== GESTIONNAIRE DYNAMIQUE DE RATE LIMITERS =====

lazy_static::lazy_static! {
    static ref RATE_LIMITERS: Arc<Mutex<HashMap<String, Arc<AdaptiveRateLimiter>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub async fn get_or_create_rate_limiter(
    source_id: &str,
    window_secs: u64,
    min_delay_ms: u64,
    burst_threshold: usize,
) -> Arc<AdaptiveRateLimiter> {
    let mut limiters = RATE_LIMITERS.lock().await;

    if let Some(limiter) = limiters.get(source_id) {
        Arc::clone(limiter)
    } else {
        let limiter = Arc::new(AdaptiveRateLimiter::new(
            window_secs,
            min_delay_ms,
            burst_threshold,
        ));
        limiters.insert(source_id.to_string(), Arc::clone(&limiter));
        log::info!("Created rate limiter for source: {}", source_id);
        limiter
    }
}
use std::collections::HashMap;
use std::time::{Duration, Instant};
use notify_rust::Notification;
use crate::app::App;
use crate::config::AlertRuleConfig;

pub struct AlertEngine {
    last_notified: HashMap<String, Instant>,
}

impl AlertEngine {
    pub fn new() -> Self {
        Self {
            last_notified: HashMap::new(),
        }
    }

    pub fn check_rules(&mut self, app: &App, rules: &[AlertRuleConfig]) {
        for rule in rules {
            let value = match rule.metric.as_str() {
                "cpu"    => app.cpu_history.iter().copied().last().unwrap_or(0) as f64,
                "memory" => (app.sys.used_memory() as f64 / app.sys.total_memory() as f64) * 100.0,
                "gpu"    => app.gpu_usage,
                "swap"   => if app.sys.total_swap() > 0 { 
                                (app.sys.used_swap() as f64 / app.sys.total_swap() as f64) * 100.0 
                            } else { 0.0 },
                _ => continue,
            };

            if value >= rule.threshold {
                self.trigger_alert(rule, value);
            }
        }
    }

    fn trigger_alert(&mut self, rule: &AlertRuleConfig, current_value: f64) {
        let key = format!("{}_{}", rule.metric, rule.threshold);
        
        if let Some(last) = self.last_notified.get(&key) {
            if last.elapsed() < Duration::from_secs(rule.cooldown_secs) {
                return;
            }
        }

        if rule.action == "notify" {
            let summary = format!("Tempest Monitor: {} Alert", rule.metric.to_uppercase());
            let body = format!(
                "Threshold exceeded: {:.1}% (Rule: {:.1}%)",
                current_value, rule.threshold
            );

            let _ = Notification::new()
                .summary(&summary)
                .body(&body)
                .icon("utilities-system-monitor")
                .timeout(Duration::from_secs(10))
                .show();
            
            log::warn!("Alert triggered: {} at {:.1}%", rule.metric, current_value);
        }

        self.last_notified.insert(key, Instant::now());
    }
}

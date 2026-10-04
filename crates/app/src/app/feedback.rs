//! The settings page's 「反馈」 page: what it may send along with the user's text (system facts
//! and the shareable log, see `crate::share_log`) and the offer to report a crash on the next
//! start. The page itself posts to the feedback service; nothing here touches the network.

use super::*;

/// The feedback service (the Cloudflare Worker in `services/feedback`), which files each report
/// as an issue in a private repository. `PECOFENCE_FEEDBACK_URL` points a test instance
/// elsewhere (e.g. `wrangler dev`).
const FEEDBACK_URL: &str = "https://api.pecofence.jiang.jp/feedback";
/// How much of each shareable log goes with a report: its last lines, up to this size.
const LOG_TAIL_BYTES: usize = 24 * 1024;

/// Feedback-page state the app keeps between page loads.
#[derive(Default)]
pub(super) struct FeedbackState {
    /// The page to show once the freshly opened settings page is ready (`Some(crash)`).
    focus: Option<bool>,
    /// The previous run ended in a panic or an unhandled exception (its shareable log says so).
    previous_run_crashed: bool,
    /// The crash balloon is up; clicking it opens the feedback page.
    crash_offer_pending: bool,
}

/// Whether a shareable log records a crash: the panic hook's `PANIC` or crashlog's line.
fn log_records_crash(log: &str) -> bool {
    log.lines().any(|line| {
        line.contains(": PANIC location=") || line.contains(": CRASH (unhandled exception)")
    })
}

/// The last whole lines of `text` within `max` bytes.
fn tail(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut start = text.len() - max;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    let cut = &text[start..];
    cut.find('\n').map(|i| &cut[i + 1..]).unwrap_or(cut)
}

impl App {
    fn share_log(&self, suffix: &str) -> Option<String> {
        let dir = crate::log_dir()?;
        let name = pecofence_core::brand::log_file_name(self.instance.as_deref(), suffix);
        std::fs::read(dir.join(name))
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }

    /// At startup: if the last run crashed, offer to report it from a tray balloon.
    pub(super) fn offer_crash_feedback(&mut self) {
        let crashed = self
            .share_log(pecofence_core::brand::PREVIOUS_SHARE_LOG)
            .is_some_and(|log| log_records_crash(&log));
        self.feedback.previous_run_crashed = crashed;
        if !crashed {
            return;
        }
        tracing::info!("previous run crashed; offering to send feedback");
        if let Some(t) = &self.tray {
            t.show_info(
                "PecoFence",
                pecofence_core::i18n::text("PecoFence 上次意外关闭了。点击这里报告问题。"),
                true,
            );
            self.feedback.crash_offer_pending = true;
        }
    }

    /// A tray balloon was clicked; only the crash offer reacts to it.
    pub(super) fn on_balloon_click(&mut self) {
        if std::mem::take(&mut self.feedback.crash_offer_pending) {
            self.open_feedback(true);
        }
    }

    /// The balloon went away unclicked (or another one replaced it).
    pub(super) fn on_balloon_gone(&mut self) {
        self.feedback.crash_offer_pending = false;
    }

    /// Opens the settings window on the 「反馈」 page; `crash` says it reports the last crash.
    pub(super) fn open_feedback(&mut self, crash: bool) {
        let already_open = self.settings.is_some();
        self.open_settings();
        if self.settings.is_none() {
            return;
        }
        if already_open {
            self.post_show_feedback(crash);
        } else {
            // The page is still loading; `ready` delivers the state and then this.
            self.feedback.focus = Some(crash);
        }
    }

    /// `ready` from a freshly loaded page: show the feedback page if that is why it opened.
    pub(super) fn show_pending_feedback(&mut self) {
        if let Some(crash) = self.feedback.focus.take() {
            self.post_show_feedback(crash);
        }
    }

    fn post_show_feedback(&self, crash: bool) {
        if let Some(h) = &self.settings {
            h.post_json(&serde_json::json!({ "type": "showFeedback", "crash": crash }).to_string());
        }
    }

    /// What the page may send besides the user's own text, shown to them before sending.
    fn feedback_info_json(&self) -> serde_json::Value {
        let install = if pecofence_platform::process::is_packaged() {
            "Microsoft Store"
        } else {
            "ZIP / winget"
        };
        let displays: Vec<String> = monitors::enumerate()
            .iter()
            .map(|m| format!("{:.0}%", m.scale() * 100.0))
            .collect();
        let settings = &self.state.config.settings;
        let mem_mb = pecofence_platform::memstats::MemoryStats::current()
            .map(|m| m.private_working_set as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);
        let system = serde_json::json!({
            "app": format!("PecoFence {} ({install})", env!("CARGO_PKG_VERSION")),
            "windows": format!("build {}", pecofence_platform::desktop::os_build()),
            "displays": displays.join(", "),
            "language": pecofence_core::i18n::language().tag(),
            "theme": format!(
                "{:?}, {}",
                settings.theme_style,
                if self.theme_mode == ThemeMode::Dark { "dark" } else { "light" }
            ),
            "workspace": format!(
                "{} fences, {} items",
                self.state.fences().len(),
                self.state.workspace_item_count()
            ),
            "memory": format!("{mem_mb:.1} MB"),
        });
        let mut log = String::new();
        if self.feedback.previous_run_crashed
            && let Some(previous) = self.share_log(pecofence_core::brand::PREVIOUS_SHARE_LOG)
        {
            log.push_str("--- previous run (crashed) ---\n");
            log.push_str(tail(&previous, LOG_TAIL_BYTES));
            log.push_str("\n--- this run ---\n");
        }
        if let Some(current) = self.share_log(pecofence_core::brand::SHARE_LOG) {
            log.push_str(tail(&current, LOG_TAIL_BYTES));
        }
        let url = pecofence_core::brand::var("PECOFENCE_FEEDBACK_URL")
            .ok()
            .filter(|u| u.starts_with("https://") || u.starts_with("http://127.0.0.1:"))
            .unwrap_or_else(|| FEEDBACK_URL.to_string());
        serde_json::json!({
            "type": "feedbackInfo",
            "url": url,
            "app": env!("CARGO_PKG_VERSION"),
            "language": pecofence_core::i18n::language().tag(),
            "crashed": self.feedback.previous_run_crashed,
            "system": system,
            "log": log,
        })
    }

    /// `{"type": "action", "name": "feedback…"}` messages from the page.
    pub(super) fn on_feedback_action(&mut self, name: &str, v: &serde_json::Value) {
        match name {
            "feedbackInfo" => {
                if let Some(h) = &self.settings {
                    h.post_json(&self.feedback_info_json().to_string());
                }
            }
            "feedbackResult" => {
                let status = v.get("status").and_then(|s| s.as_u64()).unwrap_or(0);
                if (200..300).contains(&status) {
                    tracing::info!(status, "feedback sent");
                    // A crash report went out; the next one starts from this run.
                    self.feedback.previous_run_crashed = false;
                } else {
                    tracing::warn!(status, "feedback not sent");
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_lines_are_recognised() {
        assert!(log_records_crash(
            "2026-10-04T12:00:00Z ERROR pecofence: PANIC location=crates/app/src/x.rs:1:2 payload=boom\n"
        ));
        assert!(log_records_crash(
            "t ERROR pecofence_platform::crashlog: CRASH (unhandled exception) code=0xc0000005\n"
        ));
        assert!(!log_records_crash(
            "t  INFO pecofence::app: config loaded fences=7\n"
        ));
    }

    #[test]
    fn tails_start_on_a_whole_line() {
        let log = "first line\nsecond line\nthird\n";
        assert_eq!(tail(log, 100), log);
        assert_eq!(tail(log, 14), "third\n");
        assert_eq!(tail("一二三\n四五\n", 8), "四五\n");
    }
}

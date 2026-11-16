//! Ending the meeting when you leave the call. While Graphite is
//! transcribing, watch Chrome's tab strip for the Google Meet call tab. Once a call
//! tab has been seen, the meeting is over when that tab goes away (you hit Leave and
//! closed it, or Meet sends you to its home screen) and Chrome stops showing its
//! "camera and microphone recording" mark. Muting does not end it: a muted call
//! keeps its tab title and mark.

use super::ax::{self, CallApp, Limits};
use super::perms;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Wry};

/// Tab gone: the call is surely over.
const GONE_AFTER: Duration = Duration::from_secs(5);
/// Tab still there but no longer using the mic: give a muted call time to prove itself.
const IDLE_AFTER: Duration = Duration::from_secs(20);
const POLL: Duration = Duration::from_secs(2);

/// What the tab strip says about Google Meet right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeetTabs {
    /// A tab titled "Meet - <room>" exists.
    pub call_tab: bool,
    /// That tab carries Chrome's camera/microphone recording mark.
    pub in_use: bool,
}

pub fn read_tabs(titles: &[String]) -> MeetTabs {
    let mut out = MeetTabs::default();
    for t in titles {
        if t.starts_with("Meet - ") {
            out.call_tab = true;
            if t.to_lowercase().contains("recording") {
                out.in_use = true;
            }
        }
    }
    out
}

#[derive(Debug, Default)]
pub struct Watch {
    seen: bool,
    gone_since: Option<Instant>,
    idle_since: Option<Instant>,
}

impl Watch {
    /// Feed one observation; true when the call has ended and the meeting should stop.
    pub fn observe(&mut self, tabs: MeetTabs, now: Instant) -> bool {
        if tabs.in_use {
            self.seen = true;
            self.gone_since = None;
            self.idle_since = None;
            return false;
        }
        if !self.seen {
            return false;
        }
        if !tabs.call_tab {
            self.idle_since = None;
            let since = *self.gone_since.get_or_insert(now);
            return now.duration_since(since) >= GONE_AFTER;
        }
        self.gone_since = None;
        let since = *self.idle_since.get_or_insert(now);
        now.duration_since(since) >= IDLE_AFTER
    }
}

pub fn start(app: AppHandle<Wry>) {
    let _ = std::thread::Builder::new()
        .name("meet-leave".into())
        .spawn(move || tauri::async_runtime::block_on(run(app)));
}

async fn run(app: AppHandle<Wry>) {
    let mut watch = Watch::default();
    let mut was_recording = false;
    loop {
        tokio::time::sleep(POLL).await;
        let recording = crate::audio::recording_commands::is_recording().await;
        if !recording {
            was_recording = false;
            continue;
        }
        if !was_recording {
            was_recording = true;
            watch = Watch::default();
        }
        if !perms::accessibility_granted() {
            continue;
        }
        let Some(pid) = ax::find_pid(CallApp::Chrome) else { continue };
        let lim = Limits { max_depth: 14, max_nodes: 5_000, max_time: Duration::from_millis(700) };
        let windows = ax::snapshot_windows(pid, false, &lim);
        let mut titles = Vec::new();
        for w in &windows {
            w.walk(&mut |n| {
                if n.role == "AXRadioButton" && !n.title.is_empty() {
                    titles.push(n.title.clone());
                }
            });
        }
        if watch.observe(read_tabs(&titles), Instant::now()) {
            log::info!("Meet call ended: stopping the meeting");
            if let Err(e) = crate::tray::stop_recording_and_save(&app).await {
                log::warn!("Meet call ended but stopping failed: {e}");
            }
            watch = Watch::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tabs(t: &[&str]) -> MeetTabs {
        read_tabs(&t.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
    const LIVE: &str = "Meet - Product Demo Review - Camera and microphone recording";

    #[test]
    fn reads_the_call_tab_and_its_mark() {
        assert_eq!(tabs(&[LIVE, "Inbox (3)"]), MeetTabs { call_tab: true, in_use: true });
        assert_eq!(tabs(&["Meet - abc-defg-hij"]), MeetTabs { call_tab: true, in_use: false });
        assert_eq!(tabs(&["Google Meet - Pinned", "Docs"]), MeetTabs::default());
    }

    #[test]
    fn never_stops_before_a_call_was_seen() {
        let mut w = Watch::default();
        let t0 = Instant::now();
        assert!(!w.observe(MeetTabs::default(), t0));
        assert!(!w.observe(MeetTabs::default(), t0 + Duration::from_secs(600)));
    }

    #[test]
    fn closing_the_tab_ends_it_after_a_short_wait() {
        let mut w = Watch::default();
        let t0 = Instant::now();
        let live = MeetTabs { call_tab: true, in_use: true };
        assert!(!w.observe(live, t0));
        assert!(!w.observe(MeetTabs::default(), t0 + Duration::from_secs(2)));
        assert!(!w.observe(MeetTabs::default(), t0 + Duration::from_secs(5)));
        assert!(w.observe(MeetTabs::default(), t0 + Duration::from_secs(8)));
    }

    #[test]
    fn switching_tabs_or_a_blip_does_not_end_it() {
        let mut w = Watch::default();
        let t0 = Instant::now();
        let live = MeetTabs { call_tab: true, in_use: true };
        w.observe(live, t0);
        assert!(!w.observe(MeetTabs::default(), t0 + Duration::from_secs(2)));
        assert!(!w.observe(live, t0 + Duration::from_secs(4))); // back
        assert!(!w.observe(MeetTabs::default(), t0 + Duration::from_secs(6)));
        assert!(!w.observe(MeetTabs::default(), t0 + Duration::from_secs(9)));
    }

    #[test]
    fn a_lingering_tab_without_the_mark_ends_it_only_after_longer() {
        let mut w = Watch::default();
        let t0 = Instant::now();
        w.observe(MeetTabs { call_tab: true, in_use: true }, t0);
        let lingering = MeetTabs { call_tab: true, in_use: false };
        assert!(!w.observe(lingering, t0 + Duration::from_secs(3)));
        assert!(!w.observe(lingering, t0 + Duration::from_secs(19)));
        assert!(w.observe(lingering, t0 + Duration::from_secs(24)));
    }
}

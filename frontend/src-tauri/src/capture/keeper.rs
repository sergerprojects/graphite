//! Decides which captured frames of a shared screen are worth keeping.
//!
//! A frame is kept when it looks different from the last kept one, has stopped
//! changing for a moment (so a slide mid-transition is skipped), enough time has
//! passed since the last keep, and the per-meeting cap is not reached.

use super::frame::hamming;

fn words_of(text: &str) -> std::collections::HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3)
        .map(|w| w.to_lowercase())
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub struct KeepRules {
    /// Hash distance at or below which two frames count as the same picture.
    pub same_within: u32,
    /// How long a new picture must stay put before it is kept.
    pub settle_secs: f64,
    /// Minimum time between two kept pictures.
    pub min_gap_secs: f64,
    pub max_images: usize,
}

impl Default for KeepRules {
    fn default() -> Self {
        Self { same_within: 6, settle_secs: 1.0, min_gap_secs: 3.0, max_images: 60 }
    }
}

#[derive(Debug, Default)]
pub struct Keeper {
    rules: KeepRules,
    last_kept: Option<u64>,
    /// Every picture kept so far, so a slide that comes back (or a crop that flips between two
    /// versions of the same slide) is not saved again.
    seen: Vec<u64>,
    /// The words on each kept picture.
    texts: Vec<std::collections::HashSet<String>>,
    last_kept_at: f64,
    candidate: Option<(u64, f64)>,
    kept: usize,
}

impl Keeper {
    pub fn new(rules: KeepRules) -> Self {
        Self { rules, ..Default::default() }
    }

    /// Count pictures already saved (the app restarted mid-meeting).
    pub fn set_kept(&mut self, n: usize) {
        self.kept = n;
    }

    pub fn kept(&self) -> usize {
        self.kept
    }

    /// Offer the hash of the frame seen at `t` seconds. True = save this frame.
    pub fn offer(&mut self, hash: u64, t: f64) -> bool {
        if self.kept >= self.rules.max_images {
            return false;
        }
        if let Some(last) = self.last_kept {
            if hamming(hash, last) <= self.rules.same_within {
                self.candidate = None; // back to what we already have
                return false;
            }
        }
        // Seen before, earlier in this meeting: unique pictures only.
        if self.seen.iter().any(|h| hamming(hash, *h) <= self.rules.same_within.min(3)) {
            self.candidate = None;
            return false;
        }
        match self.candidate {
            Some((c, since)) if hamming(hash, c) <= self.rules.same_within => {
                let settled = t - since >= self.rules.settle_secs;
                let spaced = self.last_kept.is_none() || t - self.last_kept_at >= self.rules.min_gap_secs;
                if settled && spaced {
                    self.last_kept = Some(hash);
                    self.seen.push(hash);
                    self.last_kept_at = t;
                    self.candidate = None;
                    self.kept += 1;
                    true
                } else {
                    false
                }
            }
            _ => {
                self.candidate = Some((hash, t));
                false
            }
        }
    }

    /// True when everything readable on this picture was already on a picture kept earlier (the same
    /// slide cut a little differently). Pictures with almost no text are never called repeats.
    pub fn text_is_repeat(&self, text: &str) -> bool {
        let words = words_of(text);
        if words.len() < 8 {
            return false;
        }
        self.texts.iter().any(|old| {
            let fresh = words.iter().filter(|w| !old.contains(*w)).count();
            fresh * 10 <= words.len()
        })
    }

    /// Remember the words of a picture that was saved.
    pub fn remember_text(&mut self, text: &str) {
        self.texts.push(words_of(text));
    }

    /// The picture offered last was a repeat after all: it does not count as kept.
    pub fn undo_keep(&mut self) {
        self.kept = self.kept.saturating_sub(1);
    }

    /// Forget the previous picture (a new share started).
    pub fn reset_picture(&mut self) {
        self.last_kept = None;
        self.candidate = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: u64 = 0x0000_0000_FFFF_FFFF;
    const B: u64 = 0xFFFF_FFFF_0000_0000;
    const C: u64 = 0x0F0F_0F0F_0F0F_0F0F;

    #[test]
    fn keeps_a_picture_only_after_it_settles() {
        let mut k = Keeper::new(KeepRules::default());
        assert!(!k.offer(A, 0.0), "first sighting is only a candidate");
        assert!(!k.offer(A, 0.5), "not settled yet");
        assert!(k.offer(A, 1.1));
        assert!(!k.offer(A, 2.0), "same picture again");
        assert_eq!(k.kept(), 1);
    }

    #[test]
    fn a_slide_that_comes_back_is_not_kept_twice() {
        let mut k = Keeper::new(KeepRules::default());
        k.offer(A, 0.0);
        assert!(k.offer(A, 1.5));
        k.offer(B, 5.0);
        assert!(k.offer(B, 6.5));
        // back to the first slide, and it stays there
        k.offer(A, 10.0);
        assert!(!k.offer(A, 12.0), "already kept earlier");
        assert_eq!(k.kept(), 2);
    }

    #[test]
    fn the_same_words_cut_differently_are_a_repeat() {
        let mut k = Keeper::new(KeepRules::default());
        let slide = "Agentforce for Marketing Starter 60K org year Includes Unmetered Agentic Content Active Campaigns Planning Paths";
        assert!(!k.text_is_repeat(slide));
        k.remember_text(slide);
        assert!(k.text_is_repeat("Agentforce for Marketing Starter 60K org year Includes Unmetered Agentic Content Active Campaigns"));
        assert!(!k.text_is_repeat("Roadmap Expansion Adoption Scorecard Readiness Activation Discovery Prioritization Governance Charter"));
        assert!(!k.text_is_repeat("too short"));
    }

    #[test]
    fn a_flicker_that_never_settles_is_skipped() {
        let mut k = Keeper::new(KeepRules::default());
        for (i, h) in [A, B, A, B, A, B].into_iter().enumerate() {
            assert!(!k.offer(h, i as f64));
        }
        assert_eq!(k.kept(), 0);
    }

    #[test]
    fn next_slide_is_kept_after_the_minimum_gap() {
        let mut k = Keeper::new(KeepRules::default());
        k.offer(A, 0.0);
        assert!(k.offer(A, 2.0));
        k.offer(B, 2.5);
        assert!(!k.offer(B, 4.0), "settled but only 2s since the last keep");
        assert!(k.offer(B, 5.1));
        assert_eq!(k.kept(), 2);
    }

    #[test]
    fn returning_to_an_older_slide_is_not_kept_again() {
        let mut k = Keeper::new(KeepRules::default());
        k.offer(A, 0.0);
        k.offer(A, 2.0);
        k.offer(B, 10.0);
        assert!(k.offer(B, 12.0));
        k.offer(A, 20.0);
        assert!(!k.offer(A, 22.0), "unique pictures only: A was already kept");
    }

    #[test]
    fn stops_at_the_cap() {
        let rules = KeepRules { max_images: 2, ..KeepRules::default() };
        let mut k = Keeper::new(rules);
        k.offer(A, 0.0);
        assert!(k.offer(A, 2.0));
        k.offer(B, 10.0);
        assert!(k.offer(B, 12.0));
        k.offer(C, 20.0);
        assert!(!k.offer(C, 22.0));
        assert_eq!(k.kept(), 2);
    }

    #[test]
    fn reset_forgets_the_last_picture_but_not_what_was_kept() {
        let mut k = Keeper::new(KeepRules::default());
        k.offer(A, 0.0);
        assert!(k.offer(A, 2.0));
        k.reset_picture();
        k.offer(A, 100.0);
        assert!(!k.offer(A, 102.0), "a share that stops and starts again does not repeat a kept picture");
        k.offer(B, 110.0);
        assert!(k.offer(B, 112.0), "a new picture after the reset is kept");
    }
}

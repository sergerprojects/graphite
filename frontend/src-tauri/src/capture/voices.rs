//! The end-of-meeting voice pass.
//!
//! While a meeting runs, Graphite keeps a small journal of voice fingerprints (one per
//! stretch of call audio, no sound). When the meeting is saved this module:
//!   1. re-clusters the journal properly (average-linkage over the whole meeting, capped by
//!      how many people are on the call), which fixes one person being split in two,
//!   2. matches each voice to a name by lining its speaking times up with the call app's
//!      "who is speaking" timeline (best one-to-one assignment, not a vote),
//!   3. remembers each confidently named voice on this Mac so the next meeting can name
//!      it from the first sentence.
//! All pure functions over plain data, tested without audio.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub const FILE: &str = "voices.json";
pub const ROSTER_FILE: &str = "roster.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub t0: f64,
    pub t1: f64,
    /// The live label this stretch got ("Speaker 3").
    pub label: String,
    /// Unit-length voice fingerprint.
    pub emb: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Journal {
    pub model: String,
    pub entries: Vec<Entry>,
}

pub fn read_journal(dir: &Path) -> Option<Journal> {
    serde_json::from_slice(&std::fs::read(dir.join(FILE)).ok()?).ok()
}

pub fn write_journal(dir: &Path, j: &Journal) {
    if let Ok(bytes) = serde_json::to_vec(j) {
        let _ = std::fs::write(dir.join(FILE), bytes);
    }
}

pub fn read_roster(dir: &Path) -> Vec<String> {
    #[derive(Deserialize)]
    struct R {
        names: Vec<String>,
    }
    std::fs::read(dir.join(ROSTER_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice::<R>(&b).ok())
        .map(|r| r.names)
        .unwrap_or_default()
}

fn dot(a: &[f32], b: &[f32]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (*x as f64) * (*y as f64)).sum()
}

fn unit(v: &[f32]) -> Vec<f32> {
    let n = v.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if n == 0.0 {
        v.to_vec()
    } else {
        v.iter().map(|x| (*x as f64 / n) as f32).collect()
    }
}

/// Average-linkage clustering on cosine similarity. Returns one cluster id per input
/// (ids 0..k, in order of first appearance). Stops merging when the closest pair is
/// less similar than `merge_sim`, but never leaves more than `max_k` clusters.
pub fn recluster(embs: &[Vec<f32>], max_k: Option<usize>, merge_sim: f64) -> Vec<usize> {
    let n = embs.len();
    if n == 0 {
        return vec![];
    }
    // sum of pairwise similarities between clusters, and sizes
    let mut sums = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let s = dot(&embs[i], &embs[j]);
            sums[i][j] = s;
            sums[j][i] = s;
        }
    }
    let mut size = vec![1usize; n];
    let mut alive = vec![true; n];
    let mut members: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    let mut k = n;
    loop {
        let mut best = (f64::MIN, 0usize, 0usize);
        for i in 0..n {
            if !alive[i] {
                continue;
            }
            for j in (i + 1)..n {
                if !alive[j] {
                    continue;
                }
                let avg = sums[i][j] / (size[i] * size[j]) as f64;
                if avg > best.0 {
                    best = (avg, i, j);
                }
            }
        }
        if k < 2 {
            break;
        }
        let forced = max_k.map(|m| k > m.max(1)).unwrap_or(false);
        if best.0 < merge_sim && !forced {
            break;
        }
        let (_, i, j) = best;
        for x in 0..n {
            if alive[x] && x != i && x != j {
                sums[i][x] += sums[j][x];
                sums[x][i] = sums[i][x];
            }
        }
        size[i] += size[j];
        let moved = std::mem::take(&mut members[j]);
        members[i].extend(moved);
        alive[j] = false;
        k -= 1;
    }
    // number clusters by first appearance
    let mut first: Vec<(usize, usize)> = (0..n)
        .filter(|i| alive[*i])
        .map(|i| (members[i].iter().min().copied().unwrap_or(0), i))
        .collect();
    first.sort();
    let mut out = vec![0usize; n];
    for (id, (_, c)) in first.iter().enumerate() {
        for m in &members[*c] {
            out[*m] = id;
        }
    }
    out
}

/// Mean of unit vectors, normalised.
pub fn centroid(embs: &[&Vec<f32>]) -> Vec<f32> {
    let Some(first) = embs.first() else { return vec![] };
    let mut acc = vec![0.0f64; first.len()];
    for e in embs {
        for (a, x) in acc.iter_mut().zip(e.iter()) {
            *a += *x as f64;
        }
    }
    unit(&acc.iter().map(|x| *x as f32).collect::<Vec<_>>())
}

/// Best one-to-one assignment (maximising the total score). `scores[r][c]`.
/// Returns, per row, the chosen column or None. Hungarian algorithm, padded to square.
pub fn best_assignment(scores: &[Vec<f64>]) -> Vec<Option<usize>> {
    let rows = scores.len();
    let cols = scores.iter().map(|r| r.len()).max().unwrap_or(0);
    let n = rows.max(cols);
    if n == 0 {
        return vec![];
    }
    let big = scores.iter().flatten().cloned().fold(0.0f64, f64::max);
    // cost = big - score (padding gets cost = big, i.e. score 0)
    let cost = |r: usize, c: usize| -> f64 {
        let s = scores.get(r).and_then(|row| row.get(c)).copied().unwrap_or(0.0);
        big - s
    };
    let inf = f64::INFINITY;
    let mut u = vec![0.0; n + 1];
    let mut v = vec![0.0; n + 1];
    let mut p = vec![0usize; n + 1];
    let mut way = vec![0usize; n + 1];
    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0;
        let mut minv = vec![inf; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = inf;
            let mut j1 = 0;
            for j in 1..=n {
                if !used[j] {
                    let cur = cost(i0 - 1, j - 1) - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }
            for j in 0..=n {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut out = vec![None; rows];
    for j in 1..=n {
        let r = p[j];
        if r >= 1 && r <= rows && j <= cols {
            out[r - 1] = Some(j - 1);
        }
    }
    out
}

fn overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}

/// How long the call app showed each name speaking while each voice cluster was talking.
/// `cluster_times[c]` are that voice's (start, end) stretches; `spans` the app's timeline,
/// shifted by `lag` seconds because the on-screen marker trails speech a little.
pub fn cooccurrence(
    cluster_times: &[Vec<(f64, f64)>],
    spans: &[super::speakers::Span],
    names: &[String],
    lag: f64,
) -> Vec<Vec<f64>> {
    cluster_times
        .iter()
        .map(|times| {
            names
                .iter()
                .map(|name| {
                    spans
                        .iter()
                        .filter(|s| &s.name == name)
                        .map(|s| times.iter().map(|(a, b)| overlap(*a, *b, s.start - lag, s.end - lag)).sum::<f64>())
                        .sum()
                })
                .collect()
        })
        .collect()
}

/// A confident voice -> name pairing.
#[derive(Debug, Clone, PartialEq)]
pub struct Naming {
    pub cluster: usize,
    pub name: String,
    pub seconds: f64,
}

/// Pair voices with names (one name per voice, one voice per name). A pairing needs at
/// least `min_seconds` of evidence and must beat the voice's next-best name by `ratio`.
pub fn name_voices(scores: &[Vec<f64>], names: &[String], min_seconds: f64, ratio: f64) -> Vec<Naming> {
    let pick = best_assignment(scores);
    pick.into_iter()
        .enumerate()
        .filter_map(|(c, col)| {
            let col = col?;
            let s = scores[c][col];
            if s < min_seconds {
                return None;
            }
            let runner_up = scores[c].iter().enumerate().filter(|(j, _)| *j != col).map(|(_, v)| *v).fold(0.0, f64::max);
            (runner_up == 0.0 || s >= ratio * runner_up).then(|| Naming { cluster: c, name: names[col].clone(), seconds: s })
        })
        .collect()
}

/// Which voice cluster each transcript line belongs to: the cluster whose stretches
/// cover most of the line; for lines too short to have a fingerprint, the nearest
/// stretch in time (within `reach` seconds).
pub fn lines_to_clusters(lines: &[super::speakers::Line], entries: &[Entry], ids: &[usize], reach: f64) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for l in lines {
        let mut by_cluster: HashMap<usize, f64> = HashMap::new();
        for (e, c) in entries.iter().zip(ids) {
            let o = overlap(l.start, l.end, e.t0, e.t1);
            if o > 0.0 {
                *by_cluster.entry(*c).or_default() += o;
            }
        }
        if let Some((c, o)) = by_cluster.into_iter().max_by(|a, b| a.1.total_cmp(&b.1)) {
            if o >= 0.3 * (l.end - l.start).max(0.001) {
                out.insert(l.id.clone(), c);
                continue;
            }
        }
        let mid = (l.start + l.end) / 2.0;
        let nearest = entries
            .iter()
            .zip(ids)
            .map(|(e, c)| ((((e.t0 + e.t1) / 2.0) - mid).abs(), *c))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((d, c)) = nearest {
            if d <= reach {
                out.insert(l.id.clone(), c);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Saved voices (on this Mac only)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Profiles {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub people: HashMap<String, Vec<Vec<f32>>>,
}

const MAX_PER_PERSON: usize = 3;
const SAME_VOICE: f64 = 0.80;
const EMA: f64 = 0.2;

impl Profiles {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec(self) {
            let _ = std::fs::write(path, bytes);
        }
    }

    /// Fold a freshly confirmed voice into the person's saved voices.
    pub fn remember(&mut self, model: &str, name: &str, voice: &[f32]) {
        if self.model != model {
            self.people.clear(); // fingerprints from another model do not compare
            self.model = model.to_string();
        }
        let voice = unit(voice);
        let list = self.people.entry(name.to_string()).or_default();
        let near = list
            .iter()
            .enumerate()
            .map(|(i, v)| (i, dot(v, &voice)))
            .filter(|(_, s)| *s >= SAME_VOICE)
            .max_by(|a, b| a.1.total_cmp(&b.1));
        match near {
            Some((i, _)) => {
                let merged: Vec<f32> = list[i]
                    .iter()
                    .zip(voice.iter())
                    .map(|(old, new)| ((1.0 - EMA) * *old as f64 + EMA * *new as f64) as f32)
                    .collect();
                list[i] = unit(&merged);
            }
            None => {
                list.push(voice);
                if list.len() > MAX_PER_PERSON {
                    list.remove(0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::speakers::Span;

    /// Unit vector pointing mostly along axis `i`, nudged by `jitter`.
    fn v(i: usize, jitter: f32) -> Vec<f32> {
        let mut x = vec![0.0f32; 8];
        x[i] = 1.0;
        x[(i + 1) % 8] = jitter;
        unit(&x)
    }

    #[test]
    fn one_voice_split_in_two_is_joined() {
        // two "Speaker" labels that are really one person (close vectors) plus another person
        let embs = vec![v(0, 0.05), v(0, 0.10), v(0, 0.08), v(3, 0.05), v(3, 0.07)];
        let ids = recluster(&embs, None, 0.6);
        assert_eq!(ids, vec![0, 0, 0, 1, 1]);
    }

    #[test]
    fn the_cap_forces_merges_even_below_the_threshold() {
        let embs = vec![v(0, 0.0), v(2, 0.0), v(4, 0.0), v(6, 0.0)];
        let ids = recluster(&embs, Some(2), 0.99);
        assert_eq!(ids.iter().collect::<std::collections::HashSet<_>>().len(), 2);
    }

    #[test]
    fn different_people_stay_apart_without_a_cap() {
        let embs = vec![v(0, 0.0), v(2, 0.0), v(4, 0.0)];
        assert_eq!(recluster(&embs, None, 0.6), vec![0, 1, 2]);
    }

    #[test]
    fn empty_and_single_inputs_are_fine() {
        assert!(recluster(&[], None, 0.6).is_empty());
        assert_eq!(recluster(&[v(0, 0.0)], Some(1), 0.6), vec![0]);
    }

    #[test]
    fn assignment_is_one_to_one_and_maximises_the_total() {
        // greedy would give voice 0 -> name 0 (9) and voice 1 -> name 1 (1); the best total
        // is voice 0 -> name 1 (8) and voice 1 -> name 0 (8).
        let scores = vec![vec![9.0, 8.0], vec![8.0, 1.0]];
        assert_eq!(best_assignment(&scores), vec![Some(1), Some(0)]);
    }

    #[test]
    fn assignment_handles_more_voices_than_names() {
        let scores = vec![vec![5.0], vec![9.0], vec![1.0]];
        let a = best_assignment(&scores);
        assert_eq!(a, vec![None, Some(0), None]);
    }

    #[test]
    fn names_follow_who_was_lit_while_each_voice_talked() {
        let names = vec!["Noah".to_string(), "Lena".to_string()];
        let spans = vec![
            Span { start: 0.0, end: 20.0, name: "Noah".into() },
            Span { start: 30.0, end: 50.0, name: "Lena".into() },
        ];
        let voices = vec![vec![(1.0, 19.0)], vec![(31.0, 49.0)]];
        let scores = cooccurrence(&voices, &spans, &names, 0.0);
        let named = name_voices(&scores, &names, 5.0, 2.0);
        assert_eq!(named.len(), 2);
        assert_eq!((named[0].cluster, named[0].name.as_str()), (0, "Noah"));
        assert_eq!((named[1].cluster, named[1].name.as_str()), (1, "Lena"));
    }

    #[test]
    fn thin_or_ambiguous_evidence_names_nobody() {
        let names = vec!["Noah".to_string(), "Lena".to_string()];
        let spans = vec![
            Span { start: 0.0, end: 4.0, name: "Noah".into() },
            Span { start: 0.0, end: 4.0, name: "Lena".into() },
        ];
        let voices = vec![vec![(0.0, 4.0)]];
        let scores = cooccurrence(&voices, &spans, &names, 0.0);
        assert!(name_voices(&scores, &names, 5.0, 2.0).is_empty(), "only 4 s of evidence");
        let long = vec![
            Span { start: 0.0, end: 30.0, name: "Noah".into() },
            Span { start: 0.0, end: 30.0, name: "Lena".into() },
        ];
        let scores = cooccurrence(&[vec![(0.0, 30.0)]], &long, &names, 0.0);
        assert!(name_voices(&scores, &names, 5.0, 2.0).is_empty(), "two names lit at once: ambiguous");
    }

    #[test]
    fn saved_voices_blend_when_close_and_add_when_new() {
        let mut p = Profiles::default();
        p.remember("m", "Maya", &v(0, 0.05));
        p.remember("m", "Maya", &v(0, 0.10));
        assert_eq!(p.people["Maya"].len(), 1, "same voice is blended, not duplicated");
        p.remember("m", "Maya", &v(5, 0.0));
        assert_eq!(p.people["Maya"].len(), 2, "a clearly different sound (new device) is kept too");
        p.remember("other-model", "Maya", &v(0, 0.0));
        assert_eq!(p.people["Maya"].len(), 1, "a different model starts over");
    }

    #[test]
    fn lines_follow_the_voice_that_covers_them() {
        use crate::capture::speakers::Line;
        let line = |id: &str, a: f64, b: f64| Line { id: id.into(), start: a, end: b, speaker: "Speaker 1".into() };
        let e = |a: f64, b: f64| Entry { t0: a, t1: b, label: "x".into(), emb: vec![] };
        let entries = vec![e(0.0, 10.0), e(10.0, 20.0)];
        let ids = vec![0, 1];
        let lines = vec![line("a", 1.0, 9.0), line("b", 11.0, 19.0), line("c", 20.5, 21.5), line("d", 90.0, 91.0)];
        let m = lines_to_clusters(&lines, &entries, &ids, 7.0);
        assert_eq!(m["a"], 0);
        assert_eq!(m["b"], 1);
        assert_eq!(m["c"], 1, "a short line takes the nearest voice");
        assert!(!m.contains_key("d"), "nothing near: left alone");
    }
}

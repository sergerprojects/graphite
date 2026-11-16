//! Who is speaking right now, read from the call app's own interface.
//!
//! - Google Meet (Chrome): every participant is a tile; the speaking tile carries a CSS
//!   class that sits on exactly one tile at a time and moves as people talk. Meet's class
//!   names are scrambled and change, so we start from the names seen in other tools and
//!   also *learn* the marker: a class that is on exactly one tile whenever it is present
//!   and has been seen on at least two different tiles.
//! - Zoom: a tile's accessibility description ends ", active speaker".
//! - Microsoft Teams: the speaking tile carries the class `vdi-frame-occlusion`.
//!
//! Everything here is a pure function of the accessibility tree, so it is tested against
//! trees shaped like real dumps.

use super::ax::Node;
use std::collections::{HashMap, HashSet};

/// Classes reported by other open-source Meet tools as "this tile is speaking".
/// (`BlxGDf` is only your own mic level, so it is left out.)
const MEET_KNOWN_MARKERS: &[&str] = &["v5h6Xc", "Oaajhc"];
const TEAMS_MARKER: &str = "vdi-frame-occlusion";

/// One participant tile.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub name: String,
    pub classes: HashSet<String>,
    /// Size of the tile on screen; when a name appears twice (their video tile and the
    /// "Name (Presenting)" box) the smaller one is the video tile.
    pub area: f64,
    /// x, y, width, height in screen points.
    pub rect: [f64; 4],
}

fn rect_ok(f: &[f64; 4]) -> bool {
    let (w, h) = (f[2], f[3]);
    w >= 150.0 && h >= 100.0 && (1.0..=2.4).contains(&(w / h))
}

/// Two boxes that are the same place on screen (a few points of slack).
fn same_rect(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 3.0)
}

fn contains(outer: &[f64; 4], inner: &[f64; 4]) -> bool {
    let (cx, cy) = (inner[0] + inner[2] / 2.0, inner[1] + inner[3] / 2.0);
    cx >= outer[0] && cx <= outer[0] + outer[2] && cy >= outer[1] && cy <= outer[1] + outer[3]
}

fn plausible_name(t: &str) -> bool {
    let t = t.trim();
    let n = t.chars().count();
    (2..=60).contains(&n) && t.chars().any(|c| c.is_alphabetic()) && !t.contains('\n')
}

/// Words that mark interface text, never a person.
const UI_WORDS: &[&str] = &[
    "microphone", "camera", "settings", "audio", "video", "effects", "reactions", "captions", "caption", "present",
    "presenting", "presentation", "share", "pin", "unpin", "mute", "unmute", "raise", "leave", "more", "options", "turn", "meeting",
    "call", "chat", "people", "activities", "host", "controls", "background", "layout", "frame", "tile", "spotlight",
];

/// Does this label look like a person's name (rather than a button or text from a shared screen)?
pub fn looks_like_a_name(t: &str) -> bool {
    let words: Vec<&str> = t.split_whitespace().collect();
    if words.is_empty() || words.len() > 5 || !plausible_name(t) {
        return false;
    }
    if words.iter().any(|w| UI_WORDS.contains(&w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()))) {
        return false;
    }
    // "Copper cable upgrade" is text from a slide: most words of a name start with a capital.
    let caps = words.iter().filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)).count();
    caps * 2 >= words.len() && caps >= 1
}

fn all_classes(n: &Node, out: &mut HashSet<String>) {
    out.extend(n.dom_classes.iter().cloned());
    for c in &n.children {
        all_classes(c, out);
    }
}

/// The shared content's area: the stage minus the strip of people's tiles beside or
/// under it (their moving video would otherwise end up in every picture).
pub fn without_tile_strip(stage: [f64; 4], tiles: &[Tile]) -> [f64; 4] {
    let mut out = stage;
    let right_from = stage[0] + stage[2] * 0.6;
    let bottom_from = stage[1] + stage[3] * 0.6;
    let (right_edge, bottom_edge) = (stage[0] + stage[2] * 1.1, stage[1] + stage[3] * 1.1);

    // Tiles lined up the same way make a strip: a row shares a top edge, a column a left edge.
    // (A tile at the far right of a bottom row is NOT a column: its left edge is shared with
    // at most the tile above it.)
    let lined_up = |keys: Vec<f64>, need: usize| -> Option<f64> {
        let mut sorted = keys.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        sorted
            .iter()
            .filter(|k| keys.iter().filter(|o| (**o - **k).abs() <= 6.0).count() >= need)
            .copied()
            .reduce(f64::min)
    };
    let in_bottom: Vec<&Tile> = tiles.iter().filter(|t| t.rect[1] >= bottom_from - 4.0 && t.rect[1] < bottom_edge).collect();
    let in_right: Vec<&Tile> = tiles.iter().filter(|t| t.rect[0] >= right_from - 4.0 && t.rect[0] < right_edge).collect();
    if let Some(y) = lined_up(in_bottom.iter().map(|t| t.rect[1]).collect(), 3) {
        out[3] = (y - 2.0 - stage[1]).max(0.0);
    }
    if let Some(x) = lined_up(in_right.iter().map(|t| t.rect[0]).collect(), 3) {
        out[2] = (x - 2.0 - stage[0]).max(0.0);
    }
    out
}

/// Participant tiles of a Meet page: the smallest tile-shaped box around each name label.
pub fn meet_tiles(page: &Node) -> Vec<Tile> {
    // (tile frame key, label, label y, tile node)
    let mut found: Vec<([i64; 4], String, f64, &Node)> = Vec::new();
    fn visit<'a>(n: &'a Node, path: &mut Vec<&'a Node>, found: &mut Vec<([i64; 4], String, f64, &'a Node)>) {
        if n.role == "AXStaticText" && looks_like_a_name(&plain_name(&n.value)) {
            if let Some(tf) = n.frame {
                // nearest ancestor that looks like a tile and contains this label...
                let near = path.iter().rposition(|a| a.frame.map(|f| rect_ok(&f) && contains(&f, &tf)).unwrap_or(false));
                if let Some(mut i) = near {
                    // ...then the outermost wrapper with the same rectangle: Meet puts the
                    // "speaking" class on a wrapper around the tile, not on the tile's inner box.
                    let base = path[i].frame.unwrap();
                    while i > 0 && path[i - 1].frame.map(|f| same_rect(&f, &base)).unwrap_or(false) {
                        i -= 1;
                    }
                    let tile = path[i];
                    let f = tile.frame.unwrap();
                    let key = [f[0] as i64, f[1] as i64, f[2] as i64, f[3] as i64];
                    found.push((key, n.value.trim().to_string(), tf[1], tile));
                }
            }
        }
        path.push(n);
        for c in &n.children {
            visit(c, path, found);
        }
        path.pop();
    }
    visit(page, &mut Vec::new(), &mut found);

    // How many tiles show each label: "Pin" or "More options" repeat on most tiles and are
    // interface, names do not.
    let tile_count = found.iter().map(|f| f.0).collect::<HashSet<_>>().len();
    let mut tiles_with: HashMap<&str, HashSet<[i64; 4]>> = HashMap::new();
    for (key, text, _, _) in &found {
        tiles_with.entry(text.as_str()).or_default().insert(*key);
    }
    // One name per tile: among the labels unique to it, the one lowest on screen.
    let mut best: HashMap<[i64; 4], (String, f64, &Node)> = HashMap::new();
    for (key, text, y, node) in &found {
        if tile_count >= 3 && tiles_with[text.as_str()].len() * 2 > tile_count {
            continue;
        }
        let better = best.get(key).map(|(_, by, _)| *y > *by).unwrap_or(true);
        if better {
            best.insert(*key, (text.clone(), *y, *node));
        }
    }
    let mut tiles: Vec<Tile> = Vec::new();
    for (name, _, node) in best.values() {
        let mut classes = HashSet::new();
        all_classes(node, &mut classes);
        let f = node.frame.unwrap_or_default();
        let name = plain_name(name);
        let tile = Tile { name: name.clone(), classes, area: f[2] * f[3], rect: f };
        match tiles.iter_mut().find(|t| t.name == name) {
            Some(existing) if tile.area < existing.area => *existing = tile,
            Some(_) => {}
            None => tiles.push(tile),
        }
    }
    tiles.sort_by(|a, b| a.name.cmp(&b.name));
    tiles
}

/// "Alex Morgan (Presenting, annotating)" / "Alex Morgan (You)" -> "Alex Morgan".
fn plain_name(raw: &str) -> String {
    let t = raw.trim();
    match t.rfind(" (") {
        Some(i) if t.ends_with(')') => t[..i].trim().to_string(),
        _ => t.to_string(),
    }
}

#[derive(Debug, Default, Clone)]
struct TokenStat {
    /// Different tiles this class has been the only holder on.
    tiles: HashSet<String>,
    /// Ticks it was present on exactly one tile / on more than one.
    sole: u32,
    shared: u32,
}

/// Finds which class marks the speaking tile, and reports who is speaking each tick.
#[derive(Debug, Default)]
pub struct MarkerTracker {
    stats: HashMap<String, TokenStat>,
}

impl MarkerTracker {
    /// Names marked as speaking in this snapshot of the tiles.
    pub fn observe(&mut self, tiles: &[Tile]) -> Vec<String> {
        if tiles.len() < 2 {
            return Vec::new();
        }
        let mut holders: HashMap<&str, Vec<&str>> = HashMap::new();
        for t in tiles {
            for c in &t.classes {
                holders.entry(c.as_str()).or_default().push(t.name.as_str());
            }
        }
        for (token, who) in &holders {
            if who.len() == tiles.len() {
                continue; // on every tile: decoration
            }
            let s = self.stats.entry((*token).to_string()).or_default();
            if who.len() == 1 {
                s.sole += 1;
                s.tiles.insert(who[0].to_string());
            } else {
                s.shared += 1;
            }
        }

        // Known markers win when they are on exactly one tile.
        for k in MEET_KNOWN_MARKERS {
            if let Some(who) = holders.get(k) {
                if who.len() == 1 {
                    return vec![who[0].to_string()];
                }
            }
        }
        // Otherwise the learned marker: sole holder, seen moving between tiles, almost
        // never shared. If several qualify, the one that has moved most.
        let learned = self
            .stats
            .iter()
            .filter(|(_, s)| s.tiles.len() >= 2 && s.shared * 10 <= s.sole)
            .filter(|(tok, _)| holders.get(tok.as_str()).map(|w| w.len() == 1).unwrap_or(false))
            .max_by_key(|(_, s)| (s.tiles.len(), s.sole))
            .map(|(tok, _)| tok.clone());
        match learned {
            Some(tok) => vec![holders[tok.as_str()][0].to_string()],
            None => Vec::new(),
        }
    }
}

/// Zoom: tiles whose description ends ", active speaker" (three or more people).
pub fn zoom_speaking(window: &Node) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    window.walk(&mut |n| {
        let d = if n.desc.is_empty() { &n.title } else { &n.desc };
        if d.to_lowercase().contains("active speaker") {
            if let Some(name) = d.split(',').next().map(str::trim).filter(|s| plausible_name(s)) {
                let name = name.trim_end_matches(" (me)").trim_end_matches(" (Host)").trim().to_string();
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
    });
    out
}

fn first_text(n: &Node) -> Option<String> {
    if n.role == "AXStaticText" && plausible_name(&n.value) {
        return Some(n.value.trim().to_string());
    }
    n.children.iter().find_map(first_text)
}

/// Teams: tiles carrying `vdi-frame-occlusion`. The name comes from the tile's own
/// description ("Name, Unmuted, ...") or its first label.
pub fn teams_speaking(window: &Node) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    window.walk(&mut |n| {
        if n.dom_classes.iter().any(|c| c == TEAMS_MARKER) {
            let from_desc = [&n.desc, &n.title]
                .iter()
                .filter(|s| !s.is_empty())
                .find_map(|s| s.split(',').next().map(str::trim).filter(|x| plausible_name(x)).map(String::from));
            if let Some(name) = from_desc.or_else(|| first_text(n)) {
                let name = name.replace("(Guest)", "").replace("(Unverified)", "").trim().to_string();
                if !name.is_empty() && !out.contains(&name) {
                    out.push(name);
                }
            }
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(role: &str, value: &str, frame: Option<[f64; 4]>, classes: &[&str], kids: Vec<Node>) -> Node {
        Node {
            role: role.into(),
            value: value.into(),
            frame,
            dom_classes: classes.iter().map(|s| s.to_string()).collect(),
            children: kids,
            ..Default::default()
        }
    }

    /// A tile shaped like Meet's: a 420x340 box holding a name label and a status group.
    fn tile(x: f64, y: f64, name: &str, status: &[&str]) -> Node {
        let label = n("AXStaticText", name, Some([x + 8.0, y + 300.0, 90.0, 20.0]), &[], vec![]);
        let flag = n("AXGroup", "", Some([x + 380.0, y + 300.0, 20.0, 20.0]), status, vec![]);
        n("AXGroup", "", Some([x, y, 420.0, 340.0]), &["dkjMxf"], vec![label, flag])
    }

    fn page(tiles: Vec<Node>) -> Node {
        n("AXWebArea", "", Some([0.0, 0.0, 1700.0, 1200.0]), &[], tiles)
    }

    #[test]
    fn labels_that_are_not_people_are_dropped() {
        for ok in ["Ana Park", "Usman Nasir", "Dev", "Mary-Ann O'Neil", "Alex Morgan (You)"] {
            assert!(looks_like_a_name(&plain_name(ok)), "{ok}");
        }
        for bad in ["Copper cable upgrade", "Audio settings", "Turn on microphone", "More options", "Pin", "Presentation"] {
            assert!(!looks_like_a_name(bad), "{bad}");
        }
    }

    #[test]
    fn the_strip_of_people_is_cut_off_the_shared_area() {
        let tile = |x: f64, y: f64| Tile { name: "x".into(), classes: HashSet::new(), area: 0.0, rect: [x, y, 200.0, 120.0] };
        let stage = [0.0, 0.0, 1600.0, 1000.0];
        // a row of four tiles along the bottom, starting at y=860
        let row: Vec<Tile> = (0..4).map(|i| tile(210.0 * i as f64, 860.0)).collect();
        assert_eq!(without_tile_strip(stage, &row), [0.0, 0.0, 1600.0, 858.0]);
        // a full-height column down the right edge cuts the width only
        let column: Vec<Tile> = (0..5).map(|i| tile(1400.0, 100.0 + 200.0 * i as f64)).collect();
        assert_eq!(without_tile_strip(stage, &column), [0.0, 0.0, 1398.0, 1000.0]);
        // no strip: unchanged
        assert_eq!(without_tile_strip(stage, &[tile(300.0, 100.0), tile(600.0, 100.0)]), stage);
    }

    #[test]
    fn two_rows_along_the_bottom_never_cut_the_width() {
        // 13 people in two rows under a wide share: tiles reach the right edge, but they are rows.
        let tile = |x: f64, y: f64| Tile { name: "x".into(), classes: HashSet::new(), area: 0.0, rect: [x, y, 220.0, 130.0] };
        let stage = [0.0, 0.0, 1650.0, 1000.0];
        let mut tiles: Vec<Tile> = (0..7).map(|i| tile(230.0 * i as f64, 730.0)).collect();
        tiles.extend((0..6).map(|i| tile(230.0 * i as f64, 865.0)));
        let r = without_tile_strip(stage, &tiles);
        assert_eq!(r[2], 1650.0, "full width kept");
        assert_eq!(r[3], 728.0, "cut above the first row");
    }

    #[test]
    fn finds_tiles_and_names() {
        let p = page(vec![tile(0.0, 0.0, "Ana Park", &["JHK7jb"]), tile(430.0, 0.0, "Ben Cho", &["JHK7jb"]), tile(860.0, 0.0, "Dev Patel", &["JHK7jb"])]);
        let names: Vec<String> = meet_tiles(&p).into_iter().map(|t| t.name).collect();
        assert_eq!(names, vec!["Ana Park", "Ben Cho", "Dev Patel"]);
    }

    #[test]
    fn repeated_interface_labels_are_not_names() {
        let mk = |x: f64, name: &str| {
            let mut t = tile(x, 0.0, name, &[]);
            t.children.push(n("AXStaticText", "Pin", Some([x + 8.0, 10.0, 30.0, 14.0]), &[], vec![]));
            t
        };
        let p = page(vec![mk(0.0, "Ana Park"), mk(430.0, "Ben Cho"), mk(860.0, "Dev Patel"), mk(1290.0, "Eli Stone")]);
        let names: Vec<String> = meet_tiles(&p).into_iter().map(|t| t.name).collect();
        assert!(!names.contains(&"Pin".to_string()));
        assert_eq!(names.len(), 4);
    }

    #[test]
    fn the_marker_on_a_wrapper_around_the_tile_still_counts() {
        // Meet wraps each tile in boxes of the same size; the speaking class sits on the outer one.
        let wrap = |x: f64, name: &str, mark: bool| {
            let inner = tile(x, 0.0, name, &["JHK7jb"]);
            let classes: &[&str] = if mark { &["v5h6Xc"] } else { &[] };
            n("AXGroup", "", Some([x, 0.0, 420.0, 340.0]), classes, vec![inner])
        };
        let p = page(vec![wrap(0.0, "Ana Park", false), wrap(430.0, "Cara Diaz", true), wrap(860.0, "Dev Patel", false)]);
        let mut t = MarkerTracker::default();
        assert_eq!(t.observe(&meet_tiles(&p)), vec!["Cara Diaz".to_string()]);
    }

    #[test]
    fn a_presenting_box_and_the_video_tile_are_one_person() {
        // The speaker's video tile and the big "Name (Presenting)" region both carry the marker.
        let video = tile(0.0, 0.0, "Cara Diaz", &["JHK7jb", "v5h6Xc"]);
        let mut big = tile(0.0, 400.0, "Cara Diaz (Presenting, annotating)", &["JHK7jb", "v5h6Xc"]);
        big.frame = Some([0.0, 400.0, 1200.0, 700.0]);
        for c in big.children.iter_mut() {
            if let Some(f) = c.frame.as_mut() {
                f[1] += 380.0;
            }
        }
        let p = page(vec![video, big, tile(430.0, 0.0, "Ana Park", &["JHK7jb"]), tile(860.0, 0.0, "Dev Patel", &["JHK7jb"])]);
        let tiles = meet_tiles(&p);
        assert_eq!(tiles.iter().filter(|t| t.name == "Cara Diaz").count(), 1);
        let mut t = MarkerTracker::default();
        assert_eq!(t.observe(&tiles), vec!["Cara Diaz".to_string()]);
    }

    #[test]
    fn known_marker_names_the_speaker_at_once() {
        let p = page(vec![tile(0.0, 0.0, "Ana Park", &["JHK7jb"]), tile(430.0, 0.0, "Cara Diaz", &["JHK7jb", "v5h6Xc"]), tile(860.0, 0.0, "Dev Patel", &["JHK7jb"])]);
        let mut t = MarkerTracker::default();
        assert_eq!(t.observe(&meet_tiles(&p)), vec!["Cara Diaz".to_string()]);
    }

    #[test]
    fn learns_a_marker_that_moves_between_tiles() {
        let frame = |who: usize| {
            let mark = |i: usize| if i == who { vec!["JHK7jb", "xQzRt"] } else { vec!["JHK7jb"] };
            meet_tiles(&page(vec![tile(0.0, 0.0, "A One", &mark(0)), tile(430.0, 0.0, "B Two", &mark(1)), tile(860.0, 0.0, "C Three", &mark(2))]))
        };
        let mut t = MarkerTracker::default();
        assert!(t.observe(&frame(0)).is_empty(), "one tile is not enough to know");
        assert_eq!(t.observe(&frame(1)), vec!["B Two".to_string()]);
        assert_eq!(t.observe(&frame(2)), vec!["C Three".to_string()]);
        assert_eq!(t.observe(&frame(0)), vec!["A One".to_string()]);
    }

    #[test]
    fn a_class_stuck_on_one_tile_is_never_a_marker() {
        // e.g. something only the self tile has
        let frame = || meet_tiles(&page(vec![tile(0.0, 0.0, "A One", &["selfOnly"]), tile(430.0, 0.0, "B Two", &[]), tile(860.0, 0.0, "C Three", &[])]));
        let mut t = MarkerTracker::default();
        for _ in 0..20 {
            assert!(t.observe(&frame()).is_empty());
        }
    }

    #[test]
    fn nobody_marked_means_nobody_speaking() {
        let p = page(vec![tile(0.0, 0.0, "A One", &[]), tile(430.0, 0.0, "B Two", &[])]);
        assert!(MarkerTracker::default().observe(&meet_tiles(&p)).is_empty());
    }

    #[test]
    fn zoom_reads_the_active_speaker_description() {
        let mk = |d: &str| Node { role: "AXTabGroup".into(), desc: d.into(), ..Default::default() };
        let w = Node {
            role: "AXWindow".into(),
            children: vec![mk("Dana Lee, Computer audio unmuted, Video off, active speaker"), mk("Sam Ito, Computer audio muted, Video off"), mk("Priya (me), Computer audio unmuted")],
            ..Default::default()
        };
        assert_eq!(zoom_speaking(&w), vec!["Dana Lee".to_string()]);
    }

    #[test]
    fn teams_reads_the_speaking_class() {
        let tile = Node {
            role: "AXGroup".into(),
            dom_classes: vec!["vdi-frame-occlusion".into()],
            desc: "Alex Rivera (Guest), Unmuted, Video off".into(),
            ..Default::default()
        };
        let w = Node { role: "AXWindow".into(), children: vec![tile], ..Default::default() };
        assert_eq!(teams_speaking(&w), vec!["Alex Rivera".to_string()]);
    }
}

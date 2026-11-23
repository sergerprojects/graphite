//! Calendar source: your Google Calendar through the Claude connector that is already
//! signed in to your Google Workspace. Graphite asks your own Claude Code CLI (headless,
//! only the calendar tools allowed) for your meetings and reads the JSON it returns.
//! No Google Cloud project, no OAuth client, no key in this app.
//!
//! The model makes one `list_events` call and Graphite lifts the connector's RAW result
//! from Claude Code's stream output, so the model never rewrites your calendar.
//! Needs the Google Calendar connector authorized once (run `/mcp` in Claude Code).
//! Each refresh takes ~15-30 s, so results are cached for several minutes.

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const CACHE_TTL: Duration = Duration::from_secs(8 * 60);
const FETCH_TIMEOUT: Duration = Duration::from_secs(150);

struct Cache {
    fetched: Instant,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    events: Vec<Value>,
}

pub struct ClaudeCalendar {
    cache: Mutex<Option<Cache>>,
}

impl ClaudeCalendar {
    pub fn new() -> Self {
        Self { cache: Mutex::new(None) }
    }

    /// Forget the cached list so the next call asks Google again.
    pub async fn invalidate(&self) {
        *self.cache.lock().await = None;
    }

    /// Events overlapping `[min, max)`. Served from cache when it covers the window and is fresh.
    pub async fn list_events(&self, min: DateTime<Utc>, max: DateTime<Utc>) -> Result<Vec<Value>> {
        let mut guard = self.cache.lock().await;
        if let Some(c) = guard.as_ref() {
            if c.fetched.elapsed() < CACHE_TTL && c.from <= min && c.to >= max {
                return Ok(overlapping(&c.events, min, max));
            }
        }
        // Fetch a generous window so the poller's next minutes are all cache hits.
        let (from, to) = (min.min(Utc::now() - ChronoDuration::hours(1)), max.max(Utc::now() + ChronoDuration::hours(48)));
        let events = fetch(from, to).await?;
        let out = overlapping(&events, min, max);
        *guard = Some(Cache { fetched: Instant::now(), from, to, events });
        Ok(out)
    }
}

/// The Google Calendar tools a Claude Code setup may have, in the order we try them.
/// People install Google through different connectors, so the tool's name (and answer) differs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    /// `list_events`: answers with Google Calendar v3 JSON.
    Json,
    /// `get_events` of the Google Workspace connector: answers with a formatted text list.
    Text,
}

const CANDIDATES: &[(&str, Kind)] = &[
    ("mcp__google-calendar__list_events", Kind::Json),
    ("mcp__google-workspace__get_events", Kind::Text),
    // The Google Workspace plugin (DevBar's "vmcp-google-workspace").
    ("mcp__plugin_google-workspace_vmcp-google-workspace__get_events", Kind::Text),
];

/// Which kind of answer a calendar tool gives, judged by its name. Any `list_events` is Google Calendar
/// v3 JSON; any `get_events` is the Workspace connector's text list.
pub(crate) fn kind_for(tool: &str) -> Option<Kind> {
    if !tool.starts_with("mcp__") {
        return None;
    }
    if let Some(k) = CANDIDATES.iter().find(|(t, _)| *t == tool).map(|(_, k)| *k) {
        return Some(k);
    }
    if tool.ends_with("__list_events") {
        Some(Kind::Json)
    } else if tool.ends_with("__get_events") {
        Some(Kind::Text)
    } else {
        None
    }
}

pub(crate) fn is_known_tool(tool: &str) -> bool {
    CANDIDATES.iter().any(|(t, _)| *t == tool)
}

/// The tool Graphite's setup check found and proved works (also saved in settings). Tried before the
/// built-in list, so a connector with a name Graphite has never seen still works.
static PREFERRED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

pub(crate) fn set_preferred(tool: Option<String>) {
    if let Ok(mut g) = PREFERRED.lock() {
        *g = tool;
    }
}

pub(crate) fn preferred() -> Option<String> {
    PREFERRED.lock().ok().and_then(|g| g.clone())
}

/// Read events once through one named tool: proof that the whole path works. Returns how many.
pub(crate) async fn test_fetch(tool: &str, kind: Kind, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<usize> {
    Ok(fetch_with(tool, kind, from, to).await?.len())
}

/// The tool that worked last time, tried first so a refresh is one call, not three.
static LAST_GOOD: std::sync::Mutex<Option<usize>> = std::sync::Mutex::new(None);

/// This tool is not set up in the person's Claude Code: try the next one.
fn is_missing_tool(err: &str) -> bool {
    err.contains("No such tool available") || err.contains("wasn't reachable")
}

async fn fetch(from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<Value>> {
    if let Some(tool) = preferred() {
        if let Some(kind) = kind_for(&tool) {
            match fetch_with(&tool, kind, from, to).await {
                Ok(events) => return Ok(events),
                Err(e) if is_missing_tool(&e.to_string()) => {} // fall through to the built-in list
                Err(e) => return Err(e),
            }
        }
    }
    let first = LAST_GOOD.lock().ok().and_then(|g| *g).unwrap_or(0).min(CANDIDATES.len() - 1);
    let order: Vec<usize> = std::iter::once(first).chain((0..CANDIDATES.len()).filter(|i| *i != first)).collect();
    let mut last_err: Option<anyhow::Error> = None;
    for i in order {
        let (tool, kind) = CANDIDATES[i];
        match fetch_with(tool, kind, from, to).await {
            Ok(events) => {
                if let Ok(mut g) = LAST_GOOD.lock() {
                    *g = Some(i);
                }
                return Ok(events);
            }
            Err(e) if is_missing_tool(&e.to_string()) => last_err = Some(e),
            Err(e) => return Err(e),
        }
    }
    let _ = last_err;
    bail!("Graphite could not find a Google Calendar tool in your Claude Code. Connect one (run /mcp in Claude Code and sign in to Google Calendar or Google Workspace), then try again.")
}

async fn fetch_with(tool: &str, kind: Kind, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<Value>> {
    if kind == Kind::Text {
        let args = json!({
            "calendar_id": "primary",
            "time_min": from.to_rfc3339(),
            "time_max": to.to_rfc3339(),
            "max_results": 250,
            "detailed": true,
        });
        let raw = crate::chat::call_mcp_tool(tool, &args, FETCH_TIMEOUT).await.map_err(|e| anyhow!(e))?;
        return events_from_workspace_text(&raw);
    }
    let mut all: Vec<Value> = Vec::new();
    let mut token: Option<String> = None;
    for _ in 0..6 {
        let mut args = json!({
            "startTime": from.to_rfc3339(),
            "endTime": to.to_rfc3339(),
            "orderBy": "startTime",
            "pageSize": 250,
        });
        if let Some(t) = &token {
            args["pageToken"] = json!(t);
        }
        let raw = crate::chat::call_mcp_tool(tool, &args, FETCH_TIMEOUT).await.map_err(|e| anyhow!(e))?;
        let (events, next) = events_from_result(&raw)?;
        all.extend(events);
        token = next;
        if token.is_none() {
            break;
        }
    }
    Ok(all)
}

/// "cheryl.cochran@x.com" -> "Cheryl Cochran". Only when the address is clearly first.last;
/// anything else is left for the shared parser to name.
fn name_from_email(email: &str) -> Option<String> {
    let local = email.split('@').next()?;
    let parts: Vec<&str> = local.split(|c| c == '.' || c == '_').collect();
    if parts.len() < 2 || parts.iter().any(|p| p.len() < 2 || !p.chars().all(|c| c.is_ascii_alphabetic())) {
        return None;
    }
    Some(
        parts
            .iter()
            .map(|p| {
                let mut c = p.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// The Google Workspace connector's `get_events` answer (a formatted list, with `detailed` on)
/// as Google Calendar v3-shaped events, so the shared parser reads them like any other source.
pub fn events_from_workspace_text(raw: &str) -> Result<Vec<Value>> {
    let mut text = raw.trim().to_string();
    // Some hosts wrap the text as {"result": "..."}.
    if text.starts_with('{') {
        if let Some(inner) = serde_json::from_str::<Value>(&text).ok().and_then(|v| v.get("result").and_then(|r| r.as_str()).map(String::from)) {
            text = inner;
        }
    }
    let owner = regex::Regex::new(r"from calendar '[^']*' for ([^\s:]+):")
        .ok()
        .and_then(|re| re.captures(&text).map(|c| c[1].to_lowercase()));
    let header = regex::Regex::new(r#"^- "(.*)" \(Starts: (\S+), Ends: (\S+)\)\s*$"#).map_err(|e| anyhow!(e))?;
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    for line in text.lines() {
        if header.is_match(line) {
            blocks.push(vec![line]);
        } else if let Some(b) = blocks.last_mut() {
            b.push(line);
        }
    }
    if blocks.is_empty() {
        let t = text.to_lowercase();
        if t.contains("no events") || t.contains("retrieved 0 events") || t.contains("found 0 events") {
            return Ok(Vec::new());
        }
        bail!("The calendar connector returned something unreadable");
    }
    let mut out = Vec::new();
    for b in blocks {
        let Some(c) = header.captures(b[0]) else { continue };
        let (summary, start, end) = (c[1].to_string(), c[2].to_string(), c[3].to_string());
        let field = |name: &str| -> Option<String> {
            b.iter()
                .skip(1)
                .find_map(|l| l.trim_start().strip_prefix(name).map(|v| v.trim().to_string()))
                .filter(|v| !v.is_empty() && !v.starts_with("No ") && v != "None")
        };
        let angle = |v: String| v.trim_matches(|c| c == '<' || c == '>').to_string();
        // "ID: abc | Link: https://..."
        let id_line = b.iter().find_map(|l| l.trim_start().strip_prefix("ID: ")).unwrap_or("");
        let (id, link) = match id_line.split_once(" | Link: ") {
            Some((i, l)) => (i.trim().to_string(), Some(l.trim().to_string())),
            None => (id_line.trim().to_string(), None),
        };
        if id.is_empty() {
            continue;
        }
        // "Attendee Details: a@x: accepted (organizer)" then indented "b@x: needsAction" lines.
        let mut attendees: Vec<Value> = Vec::new();
        let mut in_details = false;
        for l in b.iter().skip(1) {
            let t = l.trim();
            let detail = if let Some(d) = t.strip_prefix("Attendee Details:") {
                in_details = true;
                d.trim()
            } else if in_details && l.starts_with("    ") && t.contains('@') && t.contains(": ") {
                t
            } else {
                if !t.is_empty() && !l.starts_with("    ") {
                    in_details = false;
                }
                continue;
            };
            let Some((email, rest)) = detail.split_once(": ") else { continue };
            let email = email.trim().to_string();
            if !email.contains('@') {
                continue;
            }
            let status = rest.split_whitespace().next().unwrap_or("").to_string();
            let mut a = json!({
                "email": email,
                "responseStatus": status,
                "organizer": rest.contains("(organizer)"),
                "self": owner.as_deref() == Some(email.to_lowercase().as_str()),
            });
            if let Some(n) = name_from_email(&email) {
                a["displayName"] = json!(n);
            }
            attendees.push(a);
        }
        let when = |v: &str, key: &str| if v.contains('T') { json!({ key: v }) } else { json!({ "date": v }) };
        let start_v = if start.contains('T') { json!({ "dateTime": start }) } else { when(&start, "dateTime") };
        let end_v = if end.contains('T') { json!({ "dateTime": end }) } else { when(&end, "dateTime") };
        let mut ev = json!({
            "id": id,
            "summary": summary,
            "start": start_v,
            "end": end_v,
            "attendees": attendees,
        });
        if let Some(org) = field("Organizer:").map(angle) {
            ev["organizer"] = json!({ "email": org });
        }
        if let Some(l) = field("Meeting Link:") {
            ev["hangoutLink"] = json!(l);
        }
        if let Some(l) = field("Location:") {
            ev["location"] = json!(l);
        }
        if let Some(d) = field("Description:") {
            ev["description"] = json!(d);
        }
        if let Some(l) = link {
            ev["htmlLink"] = json!(l);
        }
        out.push(ev);
    }
    Ok(out)
}

/// The connector's raw `list_events` result: `{"events":[...Google Calendar v3 events...], "nextPageToken": ...}`.
pub fn events_from_result(raw: &str) -> Result<(Vec<Value>, Option<String>)> {
    let v: Value = serde_json::from_str(raw.trim()).map_err(|_| anyhow!("The calendar connector returned something unreadable"))?;
    let events = v
        .get("events")
        .and_then(|e| e.as_array())
        .ok_or_else(|| anyhow!("The calendar connector returned no event list"))?
        .clone();
    let next = v.get("nextPageToken").and_then(|t| t.as_str()).filter(|t| !t.is_empty()).map(String::from);
    Ok((events, next))
}

fn overlapping(events: &[Value], min: DateTime<Utc>, max: DateTime<Utc>) -> Vec<Value> {
    events
        .iter()
        .filter(|e| {
            let s = e.pointer("/start/dateTime").and_then(|x| x.as_str()).and_then(|x| DateTime::parse_from_rfc3339(x).ok());
            let en = e.pointer("/end/dateTime").and_then(|x| x.as_str()).and_then(|x| DateTime::parse_from_rfc3339(x).ok());
            matches!((s, en), (Some(s), Some(en)) if s.with_timezone(&Utc) < max && en.with_timezone(&Utc) > min)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKSPACE: &str = "Successfully retrieved 3 events from calendar 'primary' for alex@example.com:\n- \"Team PTO\" (Starts: 2026-11-20, Ends: 2026-12-01)\n  Description: No Description\n  Location: No Location\n  Creator: <pat@example.com>\n  Organizer: <pat@example.com>\n  Attendees: None\n  Attendee Details: None\n  ID: allday1 | Link: https://www.google.com/calendar/event?eid=aaa\n- \"Product Demo Review\" (Starts: 2026-11-23T10:00:00-05:00, Ends: 2026-11-23T10:30:00-05:00)\n  Description: Agenda inside\n  Location: Room 4\n  Creator: <pat@example.com>\n  Organizer: <pat@example.com>\n  Meeting Link: https://meet.google.com/abc-defg-hij\n  Attendees: pat@example.com, alex@example.com, sam.rivera@example.com\n  Attendee Details: pat@example.com: accepted (organizer)\n    alex@example.com: needsAction\n    sam.rivera@example.com: declined\n  ID: evt2_20261123T150000Z | Link: https://www.google.com/calendar/event?eid=bbb\n- \"Quote \"Review\"\" (Starts: 2026-11-23T12:00:00-05:00, Ends: 2026-11-23T12:30:00-05:00)\n  Attendees: None\n  Attendee Details: None\n  ID: evt3 | Link: https://www.google.com/calendar/event?eid=ccc";

    #[test]
    fn the_workspace_connector_text_becomes_calendar_events() {
        let events = events_from_workspace_text(WORKSPACE).unwrap();
        assert_eq!(events.len(), 3);
        let parsed: Vec<_> = events.iter().filter_map(crate::calendar::parse_event).collect();
        assert_eq!(parsed.len(), 2, "the all-day event is dropped by the shared parser");
        let demo = &parsed[0];
        assert_eq!(demo.summary, "Product Demo Review");
        assert_eq!(demo.video_url.as_deref(), Some("https://meet.google.com/abc-defg-hij"));
        assert_eq!(demo.attendees.len(), 3);
        assert!(demo.attendees.iter().any(|a| a.is_self && a.email == "alex@example.com"));
        assert!(demo.attendees.iter().any(|a| a.name == "Sam Rivera"));
        assert!(demo.attendees.iter().any(|a| a.organizer && a.email == "pat@example.com"));
        assert_eq!(parsed[1].summary, "Quote \"Review\"");
    }

    #[test]
    fn a_declined_invitation_is_dropped_like_in_the_other_sources() {
        let t = WORKSPACE.replace("alex@example.com: needsAction", "alex@example.com: declined");
        let parsed: Vec<_> = events_from_workspace_text(&t).unwrap().iter().filter_map(crate::calendar::parse_event).collect();
        assert!(parsed.iter().all(|e| e.summary != "Product Demo Review"));
    }

    #[test]
    fn empty_wrapped_and_unreadable_answers() {
        assert!(events_from_workspace_text("No events found in calendar 'primary'.").unwrap().is_empty());
        let wrapped = serde_json::json!({ "result": WORKSPACE }).to_string();
        assert_eq!(events_from_workspace_text(&wrapped).unwrap().len(), 3);
        assert!(events_from_workspace_text("Something else entirely").is_err());
    }

    #[test]
    fn a_missing_tool_moves_on_to_the_next_one() {
        assert!(is_missing_tool("The connector returned an error: <tool_use_error>Error: No such tool available: mcp__google-calendar__list_events</tool_use_error>"));
        assert!(is_missing_tool("The google-calendar connector wasn't reachable. If it needs signing in"));
        assert!(!is_missing_tool("The calendar connector returned something unreadable"));
    }

    #[test]
    fn names_from_first_dot_last_addresses_only() {
        assert_eq!(name_from_email("cheryl.cochran@example.com").as_deref(), Some("Cheryl Cochran"));
        assert_eq!(name_from_email("mbrowning@example.com"), None);
        assert_eq!(name_from_email("a.b@example.com"), None);
    }

    const RAW: &str = r#"{"events":[
      {"id":"a1","summary":"Sam / Alex","start":{"dateTime":"2026-11-13T12:00:00-05:00"},"end":{"dateTime":"2026-11-13T12:30:00-05:00"},
       "hangoutLink":"https://meet.google.com/abc-defg-hij","organizer":{"email":"you@example.com"},
       "attendees":[{"email":"sam@example.com","displayName":"Sam Rivera","responseStatus":"accepted"},
                    {"email":"you@example.com","self":true,"responseStatus":"accepted"}]},
      {"id":"allday","summary":"Holiday","start":{"date":"2026-11-13"},"end":{"date":"2026-11-14"}}],
      "nextPageToken":"abc"}"#;

    #[test]
    fn raw_result_flows_through_parse_event_unchanged() {
        let (events, next) = events_from_result(RAW).unwrap();
        assert_eq!(next.as_deref(), Some("abc"));
        assert_eq!(events.len(), 2);
        let parsed: Vec<_> = events.iter().filter_map(crate::calendar::parse_event).collect();
        assert_eq!(parsed.len(), 1, "all-day event is dropped by the shared parser");
        assert_eq!(parsed[0].video_url.as_deref(), Some("https://meet.google.com/abc-defg-hij"));
        assert_eq!(crate::calendar::one_other_name(&parsed[0]).as_deref(), Some("Sam Rivera"));
    }

    #[test]
    fn unreadable_results_are_plain_errors() {
        assert!(events_from_result("not json").is_err());
        assert!(events_from_result("{}").is_err());
    }

    #[test]
    fn overlap_filter() {
        let (events, _) = events_from_result(RAW).unwrap();
        let t = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        assert_eq!(overlapping(&events, t("2026-11-13T17:10:00Z"), t("2026-11-13T17:20:00Z")).len(), 1);
        assert!(overlapping(&events, t("2026-11-13T19:00:00Z"), t("2026-11-13T20:00:00Z")).is_empty());
    }
}

/// Manual check against your real connector (needs Claude Code signed in with a Google connector):
/// `cargo test --lib live_calendar_probe -- --ignored --nocapture`
#[cfg(test)]
mod live_probe {
    use super::*;

    #[test]
    #[ignore]
    fn live_calendar_probe() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let (from, to) = (Utc::now() - ChronoDuration::hours(1), Utc::now() + ChronoDuration::hours(48));
            for (tool, kind) in CANDIDATES {
                match fetch_with(tool, *kind, from, to).await {
                    Ok(events) => {
                        let parsed: Vec<_> = events.iter().filter_map(crate::calendar::parse_event).collect();
                        println!("{tool}: {} raw events, {} usable, {} with a meeting link", events.len(), parsed.len(), parsed.iter().filter(|e| e.video_url.is_some()).count());
                    }
                    Err(e) => println!("{tool}: {}", e.to_string().chars().take(120).collect::<String>()),
                }
            }
        });
    }
}

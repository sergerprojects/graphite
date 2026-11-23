//! Calendar setup that checks itself. Claude Code reports, the moment it starts, which connectors it
//! has and whether each is signed in (the `init` line of its stream output, free: no model call). From
//! that Graphite says exactly what is missing and what to do, and the same answer is available to a
//! person (Settings, Calendar) and to an agent (`graphite --calendar-doctor`, prints JSON).
//!
//! States: ready | no_claude | no_calendar_connector | needs_auth | claude_error | fetch_failed.

use super::claude_cal::{self, Kind};
use chrono::{Duration as ChronoDuration, Utc};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const PROBE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq)]
pub struct Server {
    pub name: String,
    pub status: String,
}

/// What Claude Code said about itself on start-up.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Probe {
    pub servers: Vec<Server>,
    pub tools: Vec<String>,
    pub api_key_source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub ok: bool,
    /// ready | no_claude | no_calendar_connector | needs_auth | claude_error | fetch_failed
    pub state: String,
    /// One plain sentence for the person.
    pub message: String,
    /// What to do, in order, one short step each. Empty when ready.
    pub steps: Vec<String>,
    /// The connector tool Graphite will use (when one was found).
    pub tool: Option<String>,
    pub server: Option<String>,
    /// Events found in the next two days by the real test fetch (only when the fetch ran).
    pub events: Option<usize>,
    pub api_key_source: Option<String>,
}

fn check(state: &str, ok: bool, message: &str, steps: &[&str]) -> Check {
    Check {
        ok,
        state: state.to_string(),
        message: message.to_string(),
        steps: steps.iter().map(|s| s.to_string()).collect(),
        tool: None,
        server: None,
        events: None,
        api_key_source: None,
    }
}

/// "claude.ai Google Calendar" and "claude_ai_Google_Calendar" are the same connector.
pub fn normalize(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

/// "mcp__google-calendar__list_events" -> "google-calendar".
fn server_of(tool: &str) -> Option<&str> {
    tool.strip_prefix("mcp__")?.split("__").next()
}

/// Read the `init` line of Claude Code's stream output.
pub fn parse_init(line: &str) -> Option<Probe> {
    let v: Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "system" || v.get("subtype")?.as_str()? != "init" {
        return None;
    }
    let servers = v
        .get("mcp_servers")
        .and_then(|s| s.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    Some(Server {
                        name: s.get("name")?.as_str()?.to_string(),
                        status: s.get("status").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let tools = v
        .get("tools")
        .and_then(|t| t.as_array())
        .map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let api_key_source = v.get("apiKeySource").and_then(|s| s.as_str()).map(String::from);
    Some(Probe { servers, tools, api_key_source })
}

/// The calendar tools in a probe, best first: the ones Graphite already knows, then any other
/// `list_events` / `get_events` (a connector installed under a name Graphite has never seen).
fn calendar_tools(probe: &Probe) -> Vec<(String, Kind)> {
    let mut found: Vec<(String, Kind)> = probe
        .tools
        .iter()
        .filter_map(|t| claude_cal::kind_for(t).map(|k| (t.clone(), k)))
        .collect();
    let rank = |t: &str, k: Kind| {
        let known = claude_cal::is_known_tool(t) as u8;
        let json = (k == Kind::Json) as u8;
        std::cmp::Reverse((known, json))
    };
    found.sort_by_key(|(t, k)| rank(t, *k));
    found
}

fn status_of<'a>(probe: &'a Probe, server_tool_prefix: &str) -> Option<&'a str> {
    let want = normalize(server_tool_prefix);
    probe.servers.iter().find(|s| normalize(&s.name) == want).map(|s| s.status.as_str())
}

/// Turn what Claude Code reported into a diagnosis. Pure, so every case is tested.
pub fn diagnose(probe: &Probe) -> Check {
    let key = probe.api_key_source.clone();
    let tools = calendar_tools(probe);
    // A tool whose server says it is connected (or says nothing) is usable.
    let usable = tools
        .iter()
        .find(|(t, _)| server_of(t).map(|s| status_of(probe, s).map(|st| st == "connected").unwrap_or(true)).unwrap_or(true));
    if let Some((tool, _)) = usable {
        let mut c = check("ready", true, "Your Claude Code can read your Google Calendar.", &[]);
        c.server = server_of(tool).map(String::from);
        c.tool = Some(tool.clone());
        c.api_key_source = key;
        return c;
    }

    // Something calendar-like is installed but not signed in.
    let sleepy = probe.servers.iter().find(|s| {
        let n = normalize(&s.name);
        (n.contains("calendar") || n.contains("google_workspace") || n.contains("gcal")) && s.status != "connected"
    });
    if let Some(s) = sleepy {
        let mut c = check(
            "needs_auth",
            false,
            &format!("Claude Code has \"{}\" but it isn't signed in to Google yet.", s.name),
            &[
                "Open Claude Code in a terminal (type: claude).",
                "Type /mcp, pick the Google Calendar entry, and choose Authenticate. Sign in with your work Google account in the browser.",
                "Come back here and press Connect my calendar again.",
            ],
        );
        c.server = Some(s.name.clone());
        c.api_key_source = key;
        return c;
    }

    // Nothing calendar-like at all.
    let mut steps: Vec<&str> = vec![
        "In Claude (claude.ai, Settings, Connectors) add the Google Calendar connector and sign in with your work Google account. Or, if your company gives you a Google Workspace connector for Claude Code, add that.",
        "Open Claude Code in a terminal (type: claude), type /mcp, and make sure it shows Google Calendar as connected.",
        "Come back here and press Connect my calendar.",
    ];
    // A Claude Code signed in with an API key (instead of a Claude account) never loads claude.ai connectors.
    let api_key = key.as_deref().map(|k| k.to_lowercase().contains("api")).unwrap_or(false);
    if api_key {
        steps.insert(
            0,
            "This Claude Code is using an API key, which turns off the connectors from your Claude account. In Claude Code type /login and sign in with your Claude account instead.",
        );
    }
    let mut c = check(
        "no_calendar_connector",
        false,
        "Claude Code is installed, but it has no Google Calendar connector.",
        &steps,
    );
    c.api_key_source = key;
    c
}

/// Start Claude Code and read what it reports about itself. Stops as soon as it has, so no model runs.
pub async fn probe() -> Result<Probe, Check> {
    let Some(claude) = crate::chat::find_claude() else {
        return Err(check(
            "no_claude",
            false,
            "Claude Code isn't installed on this computer.",
            &[
                "Install Claude Code from https://claude.com/claude-code (the Claude desktop or web app is not enough).",
                "Open a terminal, type: claude, and sign in once with your Claude account.",
                "Come back here and press Connect my calendar.",
            ],
        ));
    };
    let broke = |why: &str| {
        check(
            "claude_error",
            false,
            &format!("Graphite couldn't get an answer from Claude Code ({why})."),
            &[
                "Open a terminal and type: claude. If it asks you to sign in, do that, then quit it.",
                "Come back here and press Connect my calendar again.",
            ],
        )
    };
    let mut child = crate::chat::claude_command(
        &claude,
        &["-p", "--model", "haiku", "--setting-sources", "user", "--output-format", "stream-json", "--verbose", "--max-turns", "1"],
    )
    .spawn()
    .map_err(|e| broke(&e.to_string()))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"ok").await;
    }
    let stdout = child.stdout.take().ok_or_else(|| broke("no output"))?;
    let mut lines = BufReader::new(stdout).lines();
    let found = tokio::time::timeout(PROBE_TIMEOUT, async {
        while let Ok(Some(l)) = lines.next_line().await {
            if l.contains("\"init\"") {
                if let Some(p) = parse_init(&l) {
                    return Some(p);
                }
            }
        }
        None
    })
    .await;
    let _ = child.kill().await;
    match found {
        Ok(Some(p)) => Ok(p),
        Ok(None) => Err(broke("it ended without starting; it may need you to sign in")),
        Err(_) => Err(broke("it took too long to start")),
    }
}

/// Diagnose, and (when `fetch`) prove it end to end by reading the next two days of events.
pub async fn run_check(fetch: bool) -> Check {
    let probe = match probe().await {
        Ok(p) => p,
        Err(c) => return c,
    };
    let mut c = diagnose(&probe);
    if !c.ok || !fetch {
        return c;
    }
    let (Some(tool), Some(kind)) = (c.tool.clone(), c.tool.as_deref().and_then(claude_cal::kind_for)) else {
        return c;
    };
    match claude_cal::test_fetch(&tool, kind, Utc::now(), Utc::now() + ChronoDuration::hours(48)).await {
        Ok(n) => {
            c.events = Some(n);
            claude_cal::set_preferred(Some(tool));
            c.message = format!(
                "Connected. Graphite read your calendar ({} meeting{} in the next two days).",
                n,
                if n == 1 { "" } else { "s" }
            );
        }
        Err(e) => {
            c.ok = false;
            c.state = "fetch_failed".to_string();
            c.message = format!("Claude Code has a calendar connector, but reading it failed: {e}");
            c.steps = vec![
                "Open Claude Code in a terminal (type: claude) and type /mcp. If Google Calendar shows as disconnected, authenticate it.".to_string(),
                "Ask Claude Code: \"what is on my calendar today?\". If that fails, your company may be blocking the connector; ask IT for Google Calendar access in Claude.".to_string(),
                "Then press Connect my calendar again.".to_string(),
            ];
        }
    }
    c
}

/// `graphite --calendar-doctor [--save]`: print the diagnosis as JSON (and save it as calendar-doctor.json in Graphite's log folder) and exit 0 when ready, 1 when not.
/// Read-only: changes nothing, works whether or not the app is open.
pub fn run_cli() -> i32 {
    // The Windows app has no console of its own; borrow the one it was started from so the JSON shows.
    #[cfg(windows)]
    {
        extern "system" {
            fn AttachConsole(process_id: u32) -> i32;
        }
        // SAFETY: plain Win32 call, no pointers; failure (no parent console) is fine.
        unsafe {
            AttachConsole(u32::MAX);
        }
    }
    let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let c = rt.block_on(run_check(true));
    let mut out = serde_json::to_value(&c).unwrap_or_default();
    if c.ok && std::env::args().any(|a| a == "--save") {
        match save_choice(c.tool.as_deref()) {
            Ok(path) => {
                out["saved"] = true.into();
                out["note"] = format!("Saved to {}. Quit Graphite if it is open, then start it again to pick this up.", path.display()).into();
            }
            Err(e) => {
                out["saved"] = false.into();
                out["note"] = format!("Could not save: {e}. Press Connect my calendar in Settings, Calendar instead.").into();
            }
        }
    }
    let json = serde_json::to_string_pretty(&out).unwrap_or_default();
    println!("{json}");
    // Also leave it in a file (an agent started without a console can still read it).
    if let Some(dir) = crate::platform::log_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("calendar-doctor.json"), &json);
    }
    if c.ok { 0 } else { 1 }
}

/// Write "use the Claude connector" into Graphite's settings file, keeping every other setting.
/// For `--calendar-doctor --save`, which runs with the app closed (an open app would write its own copy over this).
fn save_choice(tool: Option<&str>) -> Result<std::path::PathBuf, String> {
    let dir = dirs::data_dir().ok_or("no settings folder")?.join("com.graphite.ai");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(super::ical::STORE_FILE);
    let mut v: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(|v: &Value| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    v["calendar_source"] = "claude".into();
    if let Some(t) = tool {
        v["calendar_claude_tool"] = t.into();
    }
    std::fs::write(&path, serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(servers: &[(&str, &str)], tools: &[&str]) -> Probe {
        Probe {
            servers: servers.iter().map(|(n, s)| Server { name: n.to_string(), status: s.to_string() }).collect(),
            tools: tools.iter().map(|t| t.to_string()).collect(),
            api_key_source: None,
        }
    }

    #[test]
    fn reads_the_init_line() {
        let line = r#"{"type":"system","subtype":"init","apiKeySource":"none","mcp_servers":[{"name":"google-calendar","status":"connected"}],"tools":["Bash","mcp__google-calendar__list_events"]}"#;
        let p = parse_init(line).unwrap();
        assert_eq!(p.servers[0].name, "google-calendar");
        assert!(p.tools.contains(&"mcp__google-calendar__list_events".to_string()));
        assert_eq!(p.api_key_source.as_deref(), Some("none"));
        assert!(parse_init(r#"{"type":"system","subtype":"hook_started"}"#).is_none());
        assert!(parse_init("not json").is_none());
    }

    #[test]
    fn a_signed_in_connector_is_ready() {
        let c = diagnose(&probe(&[("google-calendar", "connected")], &["mcp__google-calendar__list_events"]));
        assert!(c.ok);
        assert_eq!(c.state, "ready");
        assert_eq!(c.tool.as_deref(), Some("mcp__google-calendar__list_events"));
    }

    #[test]
    fn a_connector_installed_under_an_unknown_name_is_found() {
        let c = diagnose(&probe(
            &[("claude.ai Google Calendar", "connected")],
            &["mcp__claude_ai_Google_Calendar__list_events"],
        ));
        assert!(c.ok);
        assert_eq!(c.server.as_deref(), Some("claude_ai_Google_Calendar"));
    }

    #[test]
    fn json_list_events_beats_text_get_events_and_known_beats_unknown() {
        let p = probe(
            &[("google-workspace", "connected"), ("google-calendar", "connected")],
            &["mcp__google-workspace__get_events", "mcp__google-calendar__list_events"],
        );
        assert_eq!(diagnose(&p).tool.as_deref(), Some("mcp__google-calendar__list_events"));
    }

    #[test]
    fn an_unsigned_connector_says_how_to_sign_in() {
        let c = diagnose(&probe(&[("google-calendar", "needs-auth")], &[]));
        assert_eq!(c.state, "needs_auth");
        assert!(!c.ok);
        assert!(c.steps.iter().any(|s| s.contains("/mcp")));
    }

    #[test]
    fn a_tool_on_a_signed_out_server_is_not_usable() {
        let c = diagnose(&probe(&[("google-calendar", "failed")], &["mcp__google-calendar__list_events"]));
        assert_eq!(c.state, "needs_auth");
    }

    #[test]
    fn no_connector_at_all_says_to_add_one() {
        let c = diagnose(&probe(&[("slack", "connected")], &["mcp__slack__slack_send_message"]));
        assert_eq!(c.state, "no_calendar_connector");
        assert!(c.steps.iter().any(|s| s.contains("Connectors")));
    }

    #[test]
    fn an_api_key_login_is_called_out_first() {
        let mut p = probe(&[], &[]);
        p.api_key_source = Some("apiKeyHelper".to_string());
        let c = diagnose(&p);
        assert_eq!(c.state, "no_calendar_connector");
        assert!(c.steps[0].contains("/login"));
    }

    #[test]
    fn names_compare_the_same_however_they_are_spelled() {
        assert_eq!(normalize("claude.ai Google Calendar"), normalize("claude_ai_Google_Calendar"));
    }

    /// Live check against this computer's real Claude Code (run by hand: needs a signed-in connector).
    #[tokio::test]
    #[ignore]
    async fn live_calendar_doctor() {
        let c = run_check(true).await;
        println!("{}", serde_json::to_string_pretty(&c).unwrap());
    }
}

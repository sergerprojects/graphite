//! Share a meeting to Slack through the Slack connector that is already signed in to the
//! person's own Claude Code (the same route the calendar uses). Nothing is stored here and
//! no Slack app or token is added to Graphite.
//!
//! Safe by default: a post is saved as a Slack DRAFT for the person to look over and send
//! from Slack; "send now" is a separate, explicit choice made after previewing the text.

use serde::Serialize;
use serde_json::json;
use std::time::Duration;

const TOOL_TIMEOUT: Duration = Duration::from_secs(90);
const COMPOSE_TIMEOUT: Duration = Duration::from_secs(120);
/// Slack allows 5000 characters in one text element.
const MAX_POST: usize = 4500;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SlackTarget {
    pub id: String,
    pub name: String,
    /// "channel", "private" or "person".
    pub kind: String,
    pub detail: String,
}

/// The connector wraps its text answer as `{"results": "<markdown>"}`; give back the text.
fn result_text(raw: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw.trim())
        .ok()
        .and_then(|v| v.get("results").and_then(|r| r.as_str()).map(String::from))
        .unwrap_or_else(|| raw.to_string())
}

/// "1. #pace-team (C08CXKCT205) - public_channel - Creator: ..." lines.
pub fn parse_channels(raw: &str) -> Vec<SlackTarget> {
    result_text(raw)
        .lines()
        .filter_map(|line| {
            let rest = line.trim().split_once(". #")?.1;
            let (name, after) = rest.split_once(" (")?;
            let (id, tail) = after.split_once(')')?;
            if !id.starts_with('C') && !id.starts_with('G') {
                return None;
            }
            let private = tail.contains("private_channel");
            Some(SlackTarget {
                id: id.to_string(),
                name: format!("#{name}"),
                kind: if private { "private" } else { "channel" }.to_string(),
                detail: if private { "private channel".into() } else { "channel".into() },
            })
        })
        .collect()
}

/// "1. Pat Example - U0EXAMPLE01 - pat@example.com - President, Sales" lines.
pub fn parse_users(raw: &str) -> Vec<SlackTarget> {
    result_text(raw)
        .lines()
        .filter_map(|line| {
            let rest = line.trim().split_once(". ")?.1;
            let parts: Vec<&str> = rest.split(" - ").collect();
            if parts.len() < 2 || !parts[1].starts_with('U') || parts[1].contains(' ') {
                return None;
            }
            Some(SlackTarget {
                id: parts[1].to_string(),
                name: parts[0].to_string(),
                kind: "person".into(),
                detail: parts.get(3).or(parts.get(2)).map(|s| s.to_string()).unwrap_or_default(),
            })
        })
        .collect()
}

fn keywords(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_').trim_start_matches('#').to_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Channels and people matching what was typed.
#[tauri::command]
pub async fn slack_find(query: String) -> Result<Vec<SlackTarget>, String> {
    let kw = keywords(&query);
    if kw.is_empty() {
        return Ok(Vec::new());
    }
    let chan_args = json!({
        "keywords": kw, "natural_language_query": query, "limit": 8,
        "response_format": "concise", "channel_types": "public_channel,private_channel",
    });
    let user_args = json!({ "keywords": kw, "natural_language_query": query, "limit": 5, "response_format": "concise" });
    let (chans, users) = tokio::join!(
        crate::chat::call_mcp_tool("mcp__slack__slack_search_channels", &chan_args, TOOL_TIMEOUT),
        crate::chat::call_mcp_tool("mcp__slack__slack_search_users", &user_args, TOOL_TIMEOUT),
    );
    // One failing search must not hide the other; both failing is an error worth showing.
    if let (Err(e), Err(_)) = (&chans, &users) {
        return Err(e.clone());
    }
    let mut out = chans.map(|r| parse_channels(&r)).unwrap_or_default();
    out.extend(users.map(|r| parse_users(&r)).unwrap_or_default());
    Ok(out)
}

/// What to turn the meeting into.
pub fn compose_prompt(kind: &str, title: &str, attendees: &[String], body: &str) -> String {
    let who = if attendees.is_empty() { String::new() } else { format!("People: {}\n", attendees.join(", ")) };
    let task = match kind {
        "followup" => "Write a short, friendly Slack follow-up message to the people in this meeting: thank them in one line, restate what was decided, then list the next steps with the owner of each. Plain words, no jargon, no filler.",
        _ => "Write a short Slack post sharing this meeting with people who were not in it: one line on what it was, then the decisions, then the action items with owners. At most 10 lines. Plain, friendly words, no jargon.",
    };
    format!(
        "{task}\nUse standard markdown (**bold**, - bullets). Do not invent anything that is not in the notes. Output ONLY the message text, nothing before or after it.\n\nMeeting: {title}\n{who}\n--- NOTES ---\n{body}\n--- END ---"
    )
}

/// Keep a post under Slack's limit without cutting a word in half.
pub fn cap_post(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() <= MAX_POST {
        return s.to_string();
    }
    let cut: String = s.chars().take(MAX_POST - 30).collect();
    let cut = cut.rsplit_once(char::is_whitespace).map(|(a, _)| a.to_string()).unwrap_or(cut);
    format!("{}\n\n_(shortened for Slack)_", cut.trim_end())
}

/// Draft the message text. "full" is the summary as written; "short" and "followup" ask Claude.
#[tauri::command]
pub async fn slack_compose(kind: String, title: String, attendees: Vec<String>, body: String) -> Result<String, String> {
    if body.trim().is_empty() {
        return Err("There is nothing to share yet: generate the summary first.".into());
    }
    if kind == "full" {
        return Ok(cap_post(&format!("**{}**\n\n{}", title.trim(), body.trim())));
    }
    let prompt = compose_prompt(&kind, &title, &attendees, &body);
    let text = crate::chat::run_claude(&prompt, "sonnet", &[], COMPOSE_TIMEOUT).await?;
    let text = text.trim();
    if text.is_empty() {
        return Err("Claude returned an empty message. Try again.".into());
    }
    Ok(cap_post(text))
}

#[derive(Serialize)]
pub struct PostResult {
    /// A link to the Slack message or the channel holding the draft, when the connector gave one.
    pub link: Option<String>,
    pub sent: bool,
}

/// First https link in the connector's answer.
pub fn first_link(s: &str) -> Option<String> {
    let start = s.find("https://")?;
    let link: String = s[start..].chars().take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | ')' | '>' | '\\')).collect();
    Some(link.trim_end_matches(['.', ',']).to_string())
}

/// Save as a Slack draft (`send` = false) or post right away (`send` = true).
#[tauri::command]
pub async fn slack_post(channel_id: String, message: String, send: bool) -> Result<PostResult, String> {
    let id = channel_id.trim();
    if !(id.starts_with('C') || id.starts_with('G') || id.starts_with('U') || id.starts_with('D')) || id.contains(char::is_whitespace) {
        return Err("Pick a channel or a person first.".into());
    }
    if message.trim().is_empty() {
        return Err("The message is empty.".into());
    }
    let tool = if send { "mcp__slack__slack_send_message" } else { "mcp__slack__slack_send_message_draft" };
    let args = json!({ "channel_id": id, "message": cap_post(&message) });
    let raw = crate::chat::call_mcp_tool(tool, &args, TOOL_TIMEOUT).await?;
    if raw.contains("attached_draft_exists") {
        return Err("You already have a draft in that conversation. Send or delete it in Slack, then try again.".into());
    }
    Ok(PostResult { link: first_link(&raw), sent: send })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHANNELS: &str = r##"{"results":"# Search Results for: \n\n## Channels (3 results)\n1. #pace-solutions-cultural (C08CXKCT205) - public_channel - Creator: Amy B  - 2025-02-12 - Not Archived\n2. #fy27-pace-ta-dsr-requests (C0AD6AX5R7X) - private_channel - Creator: Syed K  - 2026-02-05 - Not Archived\n","pagination_info":"x"}"##;
    const USERS: &str = r##"{"results":"# Search Results for: \n\n## Users (2 results)\n1. Lee Sample - U0EXAMPLE02 - lee@example.com - VP, Customer Success Operations\n2. Pat Example - U0EXAMPLE01 - pat@example.com - President, Sales\n"}"##;

    #[test]
    fn channels_are_read_with_their_ids_and_privacy() {
        let c = parse_channels(CHANNELS);
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].id.as_str(), c[0].name.as_str(), c[0].kind.as_str()), ("C08CXKCT205", "#pace-solutions-cultural", "channel"));
        assert_eq!((c[1].id.as_str(), c[1].kind.as_str()), ("C0AD6AX5R7X", "private"));
    }

    #[test]
    fn people_are_read_with_their_titles() {
        let u = parse_users(USERS);
        assert_eq!(u.len(), 2);
        assert_eq!((u[1].id.as_str(), u[1].name.as_str(), u[1].kind.as_str()), ("U0EXAMPLE01", "Pat Example", "person"));
        assert_eq!(u[1].detail, "President, Sales");
    }

    #[test]
    fn unreadable_answers_give_nothing_not_garbage() {
        assert!(parse_channels("rate limited").is_empty());
        assert!(parse_users("{\"results\":\"No users found\"}").is_empty());
    }

    #[test]
    fn typed_words_become_search_keywords() {
        assert_eq!(keywords("  #Pace-team  Leaders! "), vec!["pace-team", "leaders"]);
        assert!(keywords("  ").is_empty());
    }

    #[test]
    fn long_posts_are_cut_at_a_word_with_a_note() {
        let long = "word ".repeat(2000);
        let out = cap_post(&long);
        assert!(out.chars().count() <= MAX_POST);
        assert!(out.ends_with("_(shortened for Slack)_"));
        assert_eq!(cap_post("  short  "), "short");
    }

    #[test]
    fn the_prompt_carries_notes_and_forbids_inventing() {
        let p = compose_prompt("short", "Weekly sync", &["Sam".into()], "Decided: ship Friday");
        assert!(p.contains("Weekly sync") && p.contains("People: Sam") && p.contains("Decided: ship Friday"));
        assert!(p.contains("Do not invent"));
        assert!(compose_prompt("followup", "x", &[], "y").contains("follow-up"));
    }

    #[test]
    fn the_link_is_lifted_from_the_connector_answer() {
        assert_eq!(first_link("{\"channel_link\":\"https://app.slack.com/client/T1/C2\"}"), Some("https://app.slack.com/client/T1/C2".into()));
        assert_eq!(first_link("Sent. See https://x.slack.com/archives/C1/p123."), Some("https://x.slack.com/archives/C1/p123".into()));
        assert_eq!(first_link("ok"), None);
    }
}

/// Manual check against your real Slack connector (read-only searches):
/// `cargo test --lib live_slack_find -- --ignored --nocapture`
#[cfg(test)]
mod live {
    use super::*;

    #[test]
    #[ignore]
    fn live_slack_find() {
        let found = tauri::async_runtime::block_on(slack_find("pace leaders".to_string()));
        println!("{found:?}");
        assert!(found.is_ok());
    }
}

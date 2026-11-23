# Graphite beta guide

Graphite listens to your calls, writes the transcript on your Mac, and drafts a summary when the meeting ends. Audio is not kept.

## Install
Paste this in Terminal (Apple Silicon Mac, macOS 14.4 or newer):

```
curl -fsSL https://raw.githubusercontent.com/sergerprojects/graphite/main/scripts/install-release.sh | bash
```

Follow the three setup screens. Then open **Settings, General** and check **Health and updates**: Claude Code should say Ready (summaries and Chat need it), and you can install the voice models there.

## Before your first real meeting
- Install [Claude Code](https://claude.com/claude-code) and sign in. Summaries, Chat and the calendar all use it; the Claude desktop or web app is not enough (Mac and Windows).
- Tell the people on the call you are taking notes. Transcribing people can need their consent where you are.
- Connect your calendar in **Settings, Calendar** to get meeting titles and the pop-up. Easiest: run `/mcp` in Claude Code, authorize the Google Calendar connector, then press **Connect my calendar**; it checks everything and says what is missing. You don't need an iCal link, which some Google Workspace admins turn off. An AI assistant can do the setup: see [AGENT-SETUP.md](AGENT-SETUP.md).

## What I would like you to try
1. A normal Google Meet: does the meeting stop on its own within about 10 seconds of you hitting Leave?
2. A group call: are people named correctly in the transcript? (Settings, Capture, "Who is speaking now?" shows what Graphite sees.)
3. **Continue** on a finished meeting: does the new part join the same meeting?
4. Notes: type `# `, `- `, `/` in the notes box.
5. Anything that annoys you.

## When something breaks
Settings, General, **Copy diagnostics**, then **Report a problem**. The report has versions and setup status, no meeting content.

## Updates
Graphite checks once a day and tells you. To update, run the install line again.

## Privacy
Transcripts and notes stay on your Mac. Summaries and Chat send the transcript text to Claude through your own Claude Code login. "Remember voices" is off by default; it saves voiceprints on your Mac only, and some places (Illinois BIPA, EU/UK GDPR) require consent for those. Ask your security or legal team before using it widely.

## Remove it
`bash scripts/uninstall.sh` (keeps your meetings) or `bash scripts/uninstall.sh --everything`.

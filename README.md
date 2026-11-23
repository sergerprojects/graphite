# Graphite

Private-first meeting notes for Mac. Graphite listens to your calls, writes the transcript on your Mac while you write notes, and drafts a summary when the meeting ends. There is no bot in the call, and no audio is kept.

- **Works with any call app.** Google Meet, Zoom, Microsoft Teams, Slack huddles: it hears your microphone and your Mac's call audio.
- **Transcribed on your Mac.** Speech recognition (NVIDIA Parakeet) runs locally. Transcripts and notes never leave the machine.
- **Real names, not "Speaker 1".** Reads who the call app marks as speaking (Meet, Zoom, Teams) through macOS Accessibility, and tells voices apart when it can't.
- **Notes you actually write.** Markdown as you type (`# `, `- `, `[] `, `/` for templates), folders, search, and a chat with the meeting while it runs (pop it out to a full window).
- **Summary when it ends.** Written through your own [Claude Code](https://claude.com/claude-code) login. Nothing else is sent anywhere.
- **Stops when you leave.** In Google Meet the meeting ends about 10 seconds after you hit Leave. It also stops on sleep, after a long silence, or when the calendar slot is over.
- **Calendar aware.** Meeting titles and attendees on your notes, a pop-up before each call with one button to join and start notes, and a menu-bar countdown.
- **Screen-share snapshots** (Google Meet, opt-in): pictures of what others share, saved with the meeting, with their text read on-device.

## Install

Requires an Apple Silicon Mac on macOS 14.4 or newer. Open Terminal and paste:

```bash
curl -fsSL https://raw.githubusercontent.com/sergerprojects/graphite/main/scripts/install-release.sh | bash
```

Graphite opens and walks you through three screens: download the speech model (about 670 MB, once), allow the microphone and audio capture, and optionally allow Accessibility and Screen Recording for speaker names and snapshots. Installing this way avoids the macOS "can't be opened" warning because the app is not distributed through the App Store. Run the same line again to update; Graphite also tells you once a day when a new version exists.

**Windows (preview).** Windows 10 or 11, 64-bit. Open PowerShell and paste:

```powershell
irm https://raw.githubusercontent.com/sergerprojects/graphite/main/scripts/install-release.ps1 | iex
```

It records, transcribes, takes notes, summarizes and chats like the Mac version. Speaker names from the call app and screen snapshots are Mac-only for now. Windows may show a SmartScreen warning because the installer is not signed: choose More info, then Run anyway. Remove it with `scripts\uninstall.ps1`. The installer is also on the [windows-beta release](https://github.com/sergerprojects/graphite/releases/tag/windows-beta).

**Claude Code is required** on Mac and Windows. Summaries, chat and the calendar all run through it, so install [Claude Code](https://claude.com/claude-code) and sign in before you start. The Claude desktop or web app is not enough. For the calendar, run `/mcp` in Claude Code, authorize the Google Calendar connector once, then choose **Claude connector** in Graphite's **Settings, Calendar**. No iCal link or Google Cloud project is needed (an iCal link is only an alternative, and some Google Workspace admins turn it off). **Connect my calendar** checks everything for you and says what is missing. Setting it up with an AI assistant? Point it at [docs/AGENT-SETUP.md](docs/AGENT-SETUP.md).

Prefer to download it yourself? Get `Graphite-macos-arm64.zip` from the [latest release](https://github.com/sergerprojects/graphite/releases/latest), unzip it into Applications, then run `xattr -cr /Applications/Graphite.app` before opening it.

After setup, open **Settings, General, Health and updates** to confirm Claude Code is ready (summaries and chat use it) and to install the optional voice models. See the [beta guide](docs/BETA.md) for a short list of things to try. To remove Graphite: `bash scripts/uninstall.sh` (keeps your meetings) or `bash scripts/uninstall.sh --everything`.

## How speaker names work

1. **The call app.** In Meet, Zoom and Teams, Graphite reads which participant the app marks as speaking, live. A voice that keeps lining up with one name gets it mid-meeting.
2. **Voices.** Call audio is split into voices with a speaker-embedding model (WeSpeaker ResNet293). Only fingerprints are kept, never audio. When the meeting is saved, the fingerprints are re-clustered over the whole meeting (never more voices than people on the call), and each voice is paired with the name that was lit while it talked.
3. **Remembered voices** (off by default). Saves confirmed voices on this Mac so the next meeting can name them sooner. Settings, Capture has "Forget remembered voices".

## Privacy and consent

- Audio is not kept unless you switch on "Keep audio files".
- Transcripts, notes, snapshots and settings stay on your Mac. There is no analytics or telemetry.
- Network use: the speech and voice models are downloaded once; a daily check for a newer Graphite release (GitHub); summaries and chat send the transcript text to Claude through your own Claude Code login. See [PRIVACY_POLICY.md](PRIVACY_POLICY.md).
- **Tell people you are transcribing them.** Recording or transcribing others can legally require their consent where you are.
- **Voiceprints.** "Remember voices" saves biometric voiceprints. Laws such as Illinois' Biometric Information Privacy Act and the EU/UK GDPR (Article 9) require consent for them, and AI note-takers have been sued over cross-meeting voiceprints. Leave it off unless you have checked with your security or legal team.

## Build from source

See [docs/BUILDING.md](docs/BUILDING.md). In short: Xcode command line tools, Rust, Node, cmake, then `bash scripts/install-mac.sh`.

## Project layout

- `frontend/`: the Next.js interface and the Tauri app (`frontend/src-tauri`: Rust audio capture, transcription, calendar, speaker naming)
- `llama-helper/`: the local-model sidecar
- `tools/`: the voice-labelling helper and offline diarization tool
- `scripts/`: install, uninstall, release build, voice-model setup

## Credits and license

Graphite is built on [Meetily](https://github.com/Zackriya-Solutions/meeting-minutes) by [Zackriya Solutions](https://www.zackriya.com) (MIT), which provides the original app, the on-device transcription pipeline and the Tauri foundation. The original copyright notice is kept in [LICENSE.md](LICENSE.md); Graphite is released under the same MIT License, with no warranty. Changes are listed in [CHANGELOG.md](CHANGELOG.md).

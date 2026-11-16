# Changelog

## 0.9.11
- Speaker names now work in a real group call: the speaking marker is found on the tile's outer wrapper, and a presenter's video tile and "Presenting" box count as one person.
- A notice while transcribing when macOS has not allowed Accessibility or Screen Recording (names and snapshots need them), with a Fix button.
- A capture log (`~/Library/Logs/Graphite/capture.log`, included in Copy diagnostics) that says what names and snapshots are doing.

## 0.9.10
- The update check no longer uses GitHub's rate-limited API, so it works on shared office networks.

## 0.9.9
- Repository and documentation cleanup: privacy policy, contributing, security and build guides rewritten for Graphite; stale upstream files removed.
- About text.

## 0.9.8
- Settings, General: **Health and updates** (update check, Claude Code status, voice-model installer, remember-voices switch, diagnostics, open data folders).
- A daily "new version available" notice. Uninstall script. macOS 14.4 or newer check. Beta guide.
- "Remember voices" is off by default.

## 0.9.7
- Better speaker naming: ResNet293 voice model, end-of-meeting re-clustering capped at the number of people on the call, best one-to-one pairing of voices with names, optional remembered voices.

## 0.9.6
- Continue joins the new recording onto the original meeting before opening it (no second meeting, no error page).
- Fewer pills after a meeting ends.

## 0.9.5
- The meeting stops and saves when you leave a Google Meet call.

## 0.9.4
- Markdown notes editor, folders, Home during a meeting with a chip to return, chat in a full window, a meeting pop-up with one-click join, Continue on a finished meeting, summaries written automatically, "transcribing" wording.

## 0.9.0 to 0.9.3
- Renamed from Meetily to Graphite. Speaker names from the call app, screen-share snapshots, calendar awareness, floating recording chip, auto-stop on sleep and silence, cleaned-up settings.

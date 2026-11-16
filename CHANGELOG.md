# Changelog

## 0.9.16
- Transcript as chat bubbles: your lines on the right, everyone else's on the left under a coloured name.
- Live naming trusts the call app's marker for each stretch of audio first, so people who speak only once are named too; a voice that sounds like someone seen earlier is the fallback.
- Who's who offers the names the call app shows (not just calendar handles).

## 0.9.15
- Live names: a line is named at once when the call app marks one person for nearly all of that audio, instead of waiting for a second sighting of the voice.

## 0.9.14
- Snapshots: a bottom row of people no longer cuts the right side off the shared slide. Only a lined-up row or column of tiles counts as a strip.

## 0.9.13
- Snapshots: a column of people down the side of a share is cut off as well as a row along the bottom.
- Speaker names: a voice that lines up with the same person twice gets their name straight away, and "Speaker N" fills in as that name (earlier lines are renamed too).
- Double-booked slots: the call on screen ("Meet - <meeting>") now decides which calendar event is meant.

## 0.9.12
- Screen-share snapshots no longer include the strip of people's video (it is cut off, so pictures show only the shared content and are kept when the content changes); shared-window matching is sturdier.
- Buttons and slide text are no longer mistaken for people's names. Unlabelled call audio is named live from who the call app marks as speaking.
- Continue on a double-booked meeting continues the one you chose.
- Images appear as a filmstrip with a "Taking snapshots" tile while the meeting runs.

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

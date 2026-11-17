# Changelog

## 0.9.28
- Fix a word everywhere: click "Fix a word", type the wrong and right spelling (Jetmine → Jumpmind) and it is changed in this meeting's transcript, notes and title in one go. It can remember the right spelling for next time.
- My words: a saved list of names and terms. Every new summary and every "Update from my edits" spells them the way you do.
- Undo: before "Update from my edits" or "Fix a word" changes your notes, the old version is kept. Click Undo to get it back (click again to switch back).

## 0.9.27
- "Update from my edits" on the summary page: fix a name or reword the notes yourself, then click it (optionally add what else should change) and Graphite rewrites the whole summary from the transcript, keeping your corrections everywhere they apply. If it fails, your previous notes come back.

## 0.9.26
- Snapshots are only taken of things that are presented. When Meet still had its presentation area open after a share ended, the pictures were just the people's faces; now a picture that is mostly video tiles is skipped.
- Delete snapshots from the strip: hover a picture and click the bin, or use Delete all (asks first).

## 0.9.25
- New summary model: Qwen 3.5 9B (Best Quality). In a blind test on three real meetings it made fewer mistakes and covered more than the 4B, at about 1.5x the time (about a minute for a 30-minute meeting). It is a 5.4 GB download and suits Macs with 16 GB or more. Pick it in Settings; the 4B stays the default.
- Summaries are stricter about who owns an action item: only names that appear in the transcript, "Not stated" when no owner or due date was given, no invented numbers, company names or timestamps, and ideas are no longer written up as decisions.

## 0.9.24
- A meeting keeps its calendar title (or the name you typed). Before, finishing the summary replaced it with an AI-written title, so the sidebar and the Home list disagreed. Only meetings still called "Meeting 17_11_26_..." are named by the summary now.

## 0.9.23
- The live transcript keeps following the meeting. It used to stop scrolling when an update made the page taller than 100 px at once (a long line, several lines together, or text growing in place), so you had to scroll down by hand. It now follows whenever you haven't scrolled up yourself.

## 0.9.22
- Share to Slack: a Slack button on every summary and a "Slack follow-up" chip in the live chat. Pick a channel or person (searched through the Slack connector in your Claude Code), choose Short post, Full summary or Follow-up, edit the message, then save it as a Slack draft (the default) or send it after a confirm. Nothing is stored in Graphite and no extra Slack login is needed.

## 0.9.21
- Big calls: Meet's header clock ("4:03 PM") was read as a person called "PM" and a wrapper around the whole grid inherited the speaking marker, so everything said was filed under "PM". The clock is ignored and wrapper boxes that hold other tiles are no longer treated as people.

## 0.9.20
- Leaving a Meet call ends the meeting within a few seconds: Graphite now reads Meet's own "You left the meeting" page instead of waiting 20 seconds for Chrome's recording mark to disappear.

## Unreleased
- Windows preview: the app now builds on Windows (CI builds an installer on every push), finds Claude Code and Python the Windows way, and hides the Mac-only name and snapshot settings there. PowerShell install and uninstall scripts added. Not yet tested on a real Windows machine.

## 0.9.19
- Snapshots now find the shared content from the picture itself: the people's tiles are cut off wherever they sit, because they are separated from the share by the call's dark gutter. Works for any window or monitor size and layout.

## 0.9.18
- Snapshots keep only the shared content in small calls too: two people down the side of a 1:1 are cut off, and so is Meet's floating call-controls bar and reaction tray.

## 0.9.17
- Notes feel like Notion: the same block shortcuts (⌘⌥1/2/3 headings, ⌘⇧4 to-do, ⌘⇧7 numbered, ⌘⇧8 bullets, ⌘⇧9 toggle, ⌘B/I/U/E, ⌘K link), a short "/" menu with only what a meeting needs, no side handles.
- Summaries now build on what you typed: your headings and points stay as the backbone and are expanded from the transcript, instead of a separate summary beside them. "My notes" shows alone only until a summary exists.

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

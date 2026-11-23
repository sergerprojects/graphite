# Changelog

## 0.9.39
- **Update and restart is in the menu bar.** When a newer Graphite is found, the menu shows "Update and restart (version)". One click installs it if it isn't already and restarts. During a recording the item is greyed out ("ready, after your meeting"), so a meeting is never interrupted.

## 0.9.38
- Connecting the calendar checks itself. Settings, Calendar now has one button, **Connect my calendar**. It looks at Claude Code, says plainly what is missing (not installed, no Google Calendar connector, connector not signed in, signed in with an API key) with the exact steps, and only switches over once a real read of your calendar has worked. The iCal link moved under "Other way". "Copy these steps for my AI assistant" copies the steps as text.
- A connector Graphite has never heard of now works: it finds any calendar `list_events` or `get_events` tool in Claude Code and remembers the one that worked.
- For AI assistants: `graphite --calendar-doctor` prints the same check as JSON (exit code 0 when ready) and `--save` records the choice. See docs/AGENT-SETUP.md.

## 0.9.37
- Snapshots of a shared screen are cut correctly and kept once. Graphite asked macOS for a window picture bigger than the window really was (common on a regular, non-Retina monitor); macOS pads that with black instead of enlarging it, so every position worked out from the picture was off by a sixth. The crop then flipped between two versions of the same slide (one with a row of faces) and both were saved. Pictures are now taken at the window's real size.
- Each picture is kept once. A snapshot is no longer saved if it matches any earlier one in the meeting (not only the last), or if every word on it was already on one that was saved.

## 0.9.36
- Ask a finished meeting questions. Under the transcript there is now an "Ask about this meeting…" box. It reads the whole saved transcript (with any speaker names and word fixes) and your notes, and answers the way it does during a call: your saved question buttons, Fast, Balanced or Deep, a full-window view, and a Slack follow-up. Long calls are read whole instead of only their last hour.
- A calmer summary toolbar. One row: Regenerate (with a menu for Update from my edits, template, language and AI model), Fix a word, and on the right Copy and Slack. Undo appears only when there is something to undo, and Fix a word's confirmation has an Undo button.
- Your edits to a summary save themselves a moment after you stop typing, and when you leave the page, so there is no Save button. A small "Saving…" and "Saved" tells you.
- Finding a Slack channel or person is faster. Places you shared to before show up instantly as you type and when you open the dialog. The search of all of Slack waits for a pause (or Enter), runs both lookups together in one go, and a newer search cancels an older one. On this machine the lookup went from about 22 to about 13 seconds, and a repeat of the same search is instant. The same speed-up applies to the calendar lookup.

## 0.9.35
- The calendar now works with the Google Workspace connector, not only the Google Calendar one. People who installed Google through DevBar's "google-workspace" plugin saw "Couldn't refresh your calendar ... No such tool available". Graphite now tries the Google Calendar connector, then the Google Workspace connector, then the Workspace plugin, remembers the one that works, and says clearly when none is signed in.

## 0.9.34
- Snapshots no longer include the people's strip. In a big call Meet shows two columns of small tiles beside the shared screen; they were too small to be recognised, so they ended up in the picture. Small tiles are recognised now, and any tile that still reaches into the shared area is cut out whatever shape the strip is. If tiles still overlap what would be saved, the picture is skipped instead of saved.
- Snapshots stop cutting into dark slides: the picture is trimmed by pixels only to fine-tune the reported area, never by more than a sixth, so a dark-blue slide keeps its bottom. The call-controls bar is only cut off when it is a thin bar.
- Each snapshot now keeps a small hidden record of how it was cut (screens/.diag, newest 10), so a bad one can be explained from what really happened.

## 0.9.33
- A small brand kit (docs/BRAND.md) and a quieter look to match: warm paper background, ink text, one burgundy accent for focus and hover, hairline borders, and a serif for titles and note headings so it reads like a notebook. The blue and purple accents and gradients are gone. Selected text gets a soft pink highlighter. The download page follows the same kit.

## 0.9.32
- Graphite updates itself. It checks for a new version when it opens and every few hours, downloads it quietly and installs it (never while a meeting is recording), then offers Restart now; otherwise the new version starts the next time you open it. Updates are signed, so only builds from this project are accepted. Settings, General, Health and updates has an Update and restart button. This version must be installed once the usual way; from here on it updates itself.
- A landing page (docs/index.html) with copy-and-paste install lines for Mac and Windows.

## 0.9.31
- Continue recording no longer flashes the Home screen. Clicking it from a meeting shows a quiet "Starting…" page for the second it takes, then the live note.

## 0.9.30
- Continue recording shows the whole call: the earlier part of the transcript stays above, and the new lines follow on from where it stopped (times carry on from the end of the first part), so the live transcript reads as one call.

## 0.9.29
- A call that ends now ends the meeting, whoever started the recording. Before, only recordings Graphite started itself from a calendar event stopped when the call ended; an ad-hoc call you chose to record (a Slack huddle, a Zoom you started by hand) ran until 10 minutes of silence. Now when the app that had your microphone lets go of it and 45 seconds pass with nobody talking, Graphite stops and saves. A meeting in a room (no call app) is never stopped this way, and the 10-minute silence stop still applies.

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

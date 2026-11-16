# Graphite privacy policy

Graphite is a Mac app that runs on your computer. This describes what it does with your data.

## What stays on your Mac
- Transcripts, notes, summaries, folders, snapshots of shared screens and settings.
- Audio is analysed in memory and **not saved** unless you turn on "Keep audio files" in Settings.
- Voice fingerprints used to tell speakers apart are kept only for the length of the meeting's end-of-meeting pass, next to that meeting. If you switch on "Remember voices", fingerprints of people the call app named are also kept in `~/.graphite/voices.json` until you press "Forget remembered voices" or delete that file.
- There is no analytics, tracking or crash reporting.

## What leaves your Mac
- **Model downloads.** The speech model (Hugging Face) and voice models (GitHub) are downloaded once.
- **Update check.** Once a day Graphite asks GitHub for the latest release number. It sends no personal data.
- **Summaries and chat.** When a meeting ends, and when you ask a question in chat, the transcript text is sent to Claude through the Claude Code login on your Mac, under your own Anthropic account and terms. Nothing else is sent.
- **Calendar.** If you connect a private iCal link or Google credentials, Graphite reads your calendar from that source. It never changes events.

## Permissions
Microphone and audio capture (to hear the call). Optionally Accessibility (to read participant names and who is speaking) and Screen Recording (snapshots of shared screens). Graphite does not use Accessibility or Screen Recording to record your screen or send what it sees anywhere.

## Consent
Transcribing other people can require their consent where you live. Graphite does not announce itself to the other participants; that is up to you. Saved voiceprints are biometric data in some places (for example Illinois BIPA, the EU/UK GDPR); the feature is off by default.

## Contact
Open an issue in the project repository for questions about this policy.

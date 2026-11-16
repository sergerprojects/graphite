// Dev-only: lets the UI render in a normal browser for visual checks. Never imported in builds.
const now = Date.now();
const iso = (mins: number) => new Date(now + mins * 60000).toISOString();
const events = [
  { id: 'e1', summary: 'Weekly Staff Call', start: iso(30), end: iso(60), attendees: [{ name: 'Sam Rivera', email: 'maya@example.com', is_self: false }, { name: 'Alex Morgan', email: 'owen@example.com', is_self: true }], video_url: 'https://meet.google.com/abc-defg-hij', html_link: null, location: null, description: null, organizer: null },
  { id: 'e2', summary: 'Pat / Alex 1:1', start: iso(120), end: iso(150), attendees: [{ name: 'Pat Lee', email: 'm@example.com', is_self: false }, { name: 'Alex Morgan', email: 'owen@example.com', is_self: true }], video_url: 'https://meet.google.com/xyz', html_link: null, location: null, description: null, organizer: null },
];
const meetings = [
  { id: 'meeting-1', title: 'Product Demo Review', created_at: iso(-60) },
  { id: 'meeting-2', title: 'Sam / Alex 1:1 (biweekly)', created_at: iso(-240) },
  { id: 'meeting-3', title: 'Q4 Pricing Review', created_at: iso(-60 * 30) },
];
const folders = [{ id: 'f1', name: 'Customers' }, { id: 'f2', name: 'Team 1:1s' }];
const answers: Record<string, unknown> = {
  api_get_meetings: meetings,
  folders_list: folders,
  meeting_folders: [{ meeting_id: 'meeting-2', folder_id: 'f2' }],
  calendar_upcoming: events,
  calendar_status: { enabled: true, reason: null, source: 'claude' },
  calendar_get_record_mode: 'prompt',
  api_get_onboarding_status: { completed: true, current_step: 3 },
  get_onboarding_status: { completed: true, current_step: 3 },
  get_recording_state: { is_recording: false, is_paused: false, is_active: false, recording_duration: null, active_duration: null },
  get_live_indicator: true,
  get_notification_settings: { notification_preferences: { show_recording_started: true, show_recording_stopped: true, show_recording_paused: false, show_recording_resumed: false, show_transcription_complete: false, show_meeting_reminders: false, show_system_errors: true, audio_based_recording_enabled: false, notification_sound_enabled: true, do_not_disturb_enabled: false } },
  get_storage_locations: { database: '~/Library/Application Support/com.graphite.ai', models: '', recordings: '~/Movies/graphite-recordings' },
  claude_code_status: { installed: true, path: '/Users/x/.local/bin/claude' },
  voice_models_status: { python_env: true, model: false },
  get_remember_voices: false,
  check_for_update: { current: '0.9.7', latest: '0.9.8', newer: true, url: 'https://example.com', notes: '', command: 'curl -fsSL https://raw.githubusercontent.com/sergerprojects/graphite/main/scripts/install-release.sh | bash' },
  get_recording_preferences: { save_folder: '~/Movies/graphite-recordings', auto_save: false, file_format: 'mp4', preferred_mic_device: null, preferred_system_device: null },
  get_ignored_apps: [],
  get_silence_stop_minutes: 5,
  get_meeting_alert: true,
  capture_status: { accessibility: true, screen_recording: false, names: true, snapshots: true, own_shares: false },
  api_get_model_config: { provider: 'claude-code', model: 'sonnet', whisperModel: 'large-v3' },
  meeting_chat: 'Here is a summary:\n\n- **Goal:** scale access to demos.\n- **Next step:** legal sign-off on using real brands.\n\nLet me know if you want action items.',
};
const handler = async (cmd: string, args?: any) => {
  if (cmd === 'meeting_alert_info') return { event: events[0], recording: false };
  if (cmd === 'indicator_state') return { recording: true, paused: false, secs: 868 };
  if (cmd === 'get_onboarding_status' && localStorage.getItem('mockOnboarding') === '1') return { completed: false, current_step: Number(localStorage.getItem('mockStep') || 1) };
  if (cmd === 'get_recording_state') return { is_recording: localStorage.getItem('mockRecording') === '1', is_paused: false, is_active: true, recording_duration: 120, active_duration: 120 };
  if (cmd in answers) return answers[cmd];
  if (cmd === 'calendar_week' || cmd === 'calendar_refresh') return { events, updated_at: iso(-2), error: null, enabled: true };
  if (cmd === 'calendar_agenda') return events;
  if (cmd === 'calendar_home_meetings') return meetings.map(m => ({ id: m.id, title: m.title, when: m.created_at, attendees: ['Sam Rivera'], event_id: null }));
  if (cmd === 'get_calendar_record_mode') return 'prompt';
  if (/models|devices|list_|_list|ignored/.test(cmd)) return [];
  if (cmd.startsWith('plugin:event|listen')) return 1;
  if (cmd.startsWith('plugin:')) return null;
  return []; // forgiving default: property reads on [] are undefined, not crashes
};
(window as any).__TAURI_INTERNALS__ = {
  invoke: handler,
  transformCallback: (cb: unknown) => { const id = Math.floor(Math.random() * 1e9); (window as any)['_' + id] = cb; return id; },
  metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' }, windows: [], webviews: [] },
  convertFileSrc: (p: string) => p,
};
(window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };

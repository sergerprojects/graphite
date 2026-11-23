'use client';

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import { Switch } from '@/components/ui/switch';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  CalendarEvent,
  CalendarRecordMode,
  CalendarStatus,
  CalendarCheck,
  checkAsText,
  connectCalendar,
  diagnoseCalendar,
  formatEventTime,
  getCalendarRecordMode,
  getCalendarStatus,
  setCalendarSource,
  getIgnoredApps,
  getSilenceStopMinutes,
  setSilenceStopMinutes,
  setIgnoredApps,
  getUpcomingCalendarEvents,
  setCalendarIcalUrl,
  setCalendarRecordMode,
} from '@/lib/calendar';

const MODE_LABELS: Record<CalendarRecordMode, string> = {
  off: 'Off',
  prompt: 'Ask me',
  auto: 'Start automatically',
};

/** One slice of the calendar and auto-record settings: launch at login, calendar connection, or auto-record rules. */
export function CalendarSettings({ section }: { section: 'login' | 'calendar' | 'record' }) {
  const [status, setStatus] = useState<CalendarStatus | null>(null);
  const [mode, setMode] = useState<CalendarRecordMode>('prompt');
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [launchAtLogin, setLaunchAtLogin] = useState<boolean | null>(null);
  const [meetingAlert, setMeetingAlert] = useState<boolean | null>(null);
  const [linkInput, setLinkInput] = useState('');
  const [connecting, setConnecting] = useState(false);
  const [ignoredText, setIgnoredText] = useState('');
  const [sourceBusy, setSourceBusy] = useState(false);
  const [check, setCheck] = useState<CalendarCheck | null>(null);
  const [checking, setChecking] = useState(false);
  const [silenceMinutes, setSilenceMinutes] = useState('10');

  useEffect(() => {
    isEnabled().then(setLaunchAtLogin).catch(() => setLaunchAtLogin(null));
    invoke<boolean>('get_meeting_alert').then(setMeetingAlert).catch(() => setMeetingAlert(null));
    getIgnoredApps().then(apps => setIgnoredText(apps.join(', ')));
    getSilenceStopMinutes().then(m => setSilenceMinutes(String(m)));
  }, []);

  const handleLaunchAtLogin = async (checked: boolean) => {
    const previous = launchAtLogin;
    setLaunchAtLogin(checked);
    try {
      await (checked ? enable() : disable());
    } catch (error) {
      setLaunchAtLogin(previous);
      toast.error('Failed to change launch at login', { description: String(error) });
    }
  };

  useEffect(() => {
    let active = true;
    (async () => {
      const [s, m, upcoming] = await Promise.all([
        getCalendarStatus(),
        getCalendarRecordMode(),
        getUpcomingCalendarEvents(),
      ]);
      if (!active) return;
      setStatus(s);
      setMode(m);
      setEvents(upcoming.slice(0, 3));
    })();
    return () => {
      active = false;
    };
  }, []);

  // Not connected yet: look at Claude Code right away so the page already says what is missing.
  useEffect(() => {
    if (section !== 'calendar' || status === null || status.enabled || check) return;
    let active = true;
    diagnoseCalendar().then(c => active && setCheck(c)).catch(() => {});
    return () => {
      active = false;
    };
  }, [section, status, check]);

  const refreshEvents = async () => {
    const [s, upcoming] = await Promise.all([getCalendarStatus(), getUpcomingCalendarEvents()]);
    setStatus(s);
    setEvents(upcoming.slice(0, 3));
  };

  const handleSource = async (source: 'claude' | 'google') => {
    setSourceBusy(true);
    try {
      await setCalendarSource(source);
      toast.success(source === 'claude' ? 'Reading your calendar through Claude. The first refresh takes up to 30 seconds.' : 'Back to Google sign-in');
      setTimeout(() => refreshEvents(), 35_000);
      await refreshEvents();
    } catch (error) {
      toast.error('Could not switch', { description: String(error) });
    } finally {
      setSourceBusy(false);
    }
  };

  const handleConnectClaude = async () => {
    setChecking(true);
    try {
      const result = await connectCalendar();
      setCheck(result);
      if (result.ok) toast.success('Calendar connected');
      await refreshEvents();
    } catch (error) {
      toast.error('Could not check the calendar', { description: String(error) });
    } finally {
      setChecking(false);
    }
  };

  const handleConnect = async () => {
    setConnecting(true);
    try {
      await setCalendarIcalUrl(linkInput);
      setLinkInput('');
      toast.success('Google Calendar connected');
      await refreshEvents();
    } catch (error) {
      toast.error('Could not connect calendar', { description: String(error) });
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    try {
      await setCalendarIcalUrl(null);
      await refreshEvents();
    } catch (error) {
      toast.error('Could not remove calendar link', { description: String(error) });
    }
  };

  const handleSilenceChange = async (value: string) => {
    const previous = silenceMinutes;
    setSilenceMinutes(value);
    try {
      await setSilenceStopMinutes(Number(value));
    } catch (error) {
      setSilenceMinutes(previous);
      toast.error('Failed to save', { description: String(error) });
    }
  };

  const saveIgnoredApps = async () => {
    const apps = ignoredText.split(',').map(a => a.trim()).filter(Boolean);
    try {
      await setIgnoredApps(apps);
      setIgnoredText(apps.join(', '));
      toast.success('Saved');
    } catch (error) {
      toast.error('Failed to save', { description: String(error) });
    }
  };

  const handleModeChange = async (value: string) => {
    const next = value as CalendarRecordMode;
    const previous = mode;
    setMode(next);
    try {
      await setCalendarRecordMode(next);
    } catch (error) {
      setMode(previous);
      toast.error('Failed to save the automatic-start setting', { description: String(error) });
    }
  };

  return (
    <div className="space-y-4">
      {section === 'login' && (
        <>
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1 pr-4">
          <div className="font-medium">Launch at login</div>
          <div className="text-sm text-gray-600">
            Starts Graphite in the menu bar when you log in, so meeting prompts and automatic start work.
          </div>
        </div>
        <Switch
          checked={launchAtLogin === true}
          disabled={launchAtLogin === null}
          onCheckedChange={handleLaunchAtLogin}
        />
      </div>

      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1 pr-4">
          <div className="font-medium">Meeting pop-up</div>
          <div className="text-sm text-gray-600">
            A small pop-up shortly before a meeting with a video link, with one button to join and start notes.
          </div>
        </div>
        <Switch
          checked={meetingAlert === true}
          disabled={meetingAlert === null}
          onCheckedChange={(checked) => {
            setMeetingAlert(checked);
            invoke('set_meeting_alert', { enabled: checked }).catch(() => setMeetingAlert(!checked));
          }}
        />
      </div>

        </>
      )}

      {section === 'calendar' && (
        <>
      <div className="p-4 border rounded-lg bg-gray-50 space-y-3">
        <div className="font-medium">Google Calendar</div>
        <div className="text-sm text-gray-600 break-words">
          {status === null
            ? 'Checking…'
            : status.enabled
              ? `Connected via ${status.source === 'ical' ? 'private iCal link' : status.source === 'claude' ? 'your Claude Code Google connection' : 'Google sign-in (gcloud)'}${status.reason ? ` (last sync error: ${status.reason})` : ''}`
              : `Not connected${status.reason && status.reason !== 'Calendar not initialized' ? ` — ${status.reason}` : ''}`}
        </div>

        {status?.source !== 'ical' && (
          <div className="space-y-3 rounded-lg border border-blue-100 bg-blue-50 p-3">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm text-gray-700">
                <div className="font-medium">
                  {status?.source === 'claude' ? 'Reading your calendar through Claude Code' : 'Connect my calendar'}
                </div>
                <div className="text-xs text-gray-500">
                  Needs Claude Code installed and signed in to your Google Calendar connector. Graphite checks it for you and tells you what is missing.
                </div>
              </div>
              <Button size="sm" onClick={handleConnectClaude} disabled={checking}>
                {checking ? 'Checking… (up to 30 s)' : status?.source === 'claude' ? 'Check again' : 'Connect my calendar'}
              </Button>
            </div>

            {check && (
              <div className={`rounded-md border p-3 text-sm space-y-2 ${check.ok ? 'border-green-200 bg-green-50 text-green-800' : 'border-amber-200 bg-amber-50 text-amber-900'}`}>
                <div className="font-medium">{check.ok ? '✓ ' : ''}{check.message}</div>
                {check.steps.length > 0 && (
                  <ol className="list-decimal pl-5 space-y-1 text-xs">
                    {check.steps.map((st, i) => (
                      <li key={i}>{st}</li>
                    ))}
                  </ol>
                )}
                {!check.ok && (
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      navigator.clipboard.writeText(checkAsText(check)).then(
                        () => toast.success('Copied. Paste it to your AI assistant and it can finish the setup.'),
                        () => toast.error('Could not copy'),
                      );
                    }}
                  >
                    Copy these steps for my AI assistant
                  </Button>
                )}
              </div>
            )}

            {status?.source === 'claude' && (
              <button className="text-xs text-gray-500 underline" onClick={() => handleSource('google')} disabled={sourceBusy}>
                Stop using Claude Code for the calendar
              </button>
            )}
          </div>
        )}

        {status?.source === 'ical' ? (
          <Button variant="outline" size="sm" onClick={handleDisconnect}>
            Remove link
          </Button>
        ) : (
          <details className="text-sm">
            <summary className="cursor-pointer text-xs text-gray-500">Other way: private iCal link (some companies turn this off)</summary>
            <div className="space-y-2 pt-2">
              <p className="text-xs text-gray-500">
                In Google Calendar open Settings → your calendar → Integrate calendar and copy the{' '}
                <b>Secret address in iCal format</b>. If that box is missing, your company has turned it off; use Connect my calendar above. Treat the link like a password: anyone with it can read your calendar. It is stored only on this computer. New or moved meetings can take a few hours to appear.
              </p>
              <div className="flex gap-2">
                <Input
                  type="password"
                  autoComplete="off"
                  placeholder="https://calendar.google.com/calendar/ical/…/basic.ics"
                  value={linkInput}
                  onChange={e => setLinkInput(e.target.value)}
                />
                <Button size="sm" onClick={handleConnect} disabled={connecting || !linkInput.trim()}>
                  {connecting ? 'Checking…' : 'Connect'}
                </Button>
              </div>
            </div>
          </details>
        )}
      </div>

        </>
      )}

      {section === 'record' && (
        <>
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1 pr-4">
          <div className="font-medium">When I join a meeting</div>
          <div className="text-sm text-gray-600">
            Automatic transcribing stops when the call ends or a few minutes after the meeting&apos;s end time.
          </div>
        </div>
        <Select value={mode} onValueChange={handleModeChange}>
          <SelectTrigger className="w-[200px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {(Object.keys(MODE_LABELS) as CalendarRecordMode[]).map(m => (
              <SelectItem key={m} value={m}>
                {MODE_LABELS[m]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>


      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1 pr-4">
          <div className="font-medium">Stop when nobody is talking</div>
          <div className="text-sm text-gray-600">
            Ends and saves the meeting after this long with no speech in the transcript. Never while paused.
          </div>
        </div>
        <Select value={silenceMinutes} onValueChange={handleSilenceChange}>
          <SelectTrigger className="w-[200px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {[['0', 'Never'], ['3', 'After 3 minutes'], ['5', 'After 5 minutes'], ['10', 'After 10 minutes'], ['15', 'After 15 minutes'], ['30', 'After 30 minutes']].map(([v, label]) => (
              <SelectItem key={v} value={v}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>


      <div className="p-4 border rounded-lg space-y-2">
        <div className="font-medium">Never treat these apps as a meeting</div>
        <div className="text-sm text-gray-600">
          Comma-separated app names (for example Discord, FaceTime). Using the microphone in these apps won&apos;t offer to take notes.
        </div>
        <div className="flex gap-2">
          <Input
            placeholder="Discord, FaceTime"
            value={ignoredText}
            onChange={e => setIgnoredText(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && saveIgnoredApps()}
          />
          <Button size="sm" variant="outline" onClick={saveIgnoredApps}>
            Save
          </Button>
        </div>
      </div>

        </>
      )}
    </div>
  );
}

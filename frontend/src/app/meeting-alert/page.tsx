"use client";

import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ChevronDown, Video, X } from 'lucide-react';

interface AlertEvent {
  id: string;
  summary: string;
  start: string;
  end: string;
  video_url: string | null;
}
interface AlertInfo {
  event: AlertEvent;
  recording: boolean;
}

function provider(url: string | null): string {
  if (!url) return 'meeting';
  if (url.includes('meet.google.com')) return 'Meet';
  if (url.includes('zoom')) return 'Zoom';
  if (url.includes('teams.')) return 'Teams';
  return 'meeting';
}

function timeRange(e: AlertEvent): string {
  const fmt = (iso: string) =>
    new Date(iso).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  return `${fmt(e.start)} - ${fmt(e.end)}`;
}

/** Pop-up shown shortly before a calendar meeting with a video link. */
export default function MeetingAlertPage() {
  const [info, setInfo] = useState<AlertInfo | null>(null);
  const [menu, setMenu] = useState(false);
  const closing = useRef(false);

  useEffect(() => {
    let alive = true;
    const load = async () => {
      const next = await invoke<AlertInfo | null>('meeting_alert_info').catch(() => null);
      if (!alive || closing.current) return;
      if (!next || next.recording || new Date(next.event.end).getTime() <= Date.now()) {
        closing.current = true;
        invoke('meeting_alert_close');
        return;
      }
      setInfo(next);
    };
    load();
    const t = setInterval(load, 2000);
    const un = listen('meeting-alert-changed', load);
    return () => {
      alive = false;
      clearInterval(t);
      un.then((f) => f());
    };
  }, []);

  if (!info) return null;
  const { event } = info;
  const label = provider(event.video_url);

  const join = (record: boolean) => invoke('meeting_alert_join', { record });

  return (
    <div className="relative h-screen w-screen select-none p-3 text-gray-900">
      <div
        data-tauri-drag-region
        className="relative flex items-center gap-4 rounded-2xl border border-gray-200 bg-white px-4 py-3 shadow-lg"
      >
        <button
          aria-label="Dismiss"
          onClick={() => invoke('meeting_alert_dismiss')}
          className="absolute -left-2 -top-2 flex h-6 w-6 items-center justify-center rounded-full border border-gray-200 bg-gray-100 text-gray-600 hover:bg-gray-200"
        >
          <X className="h-3.5 w-3.5" />
        </button>
        <div data-tauri-drag-region className="h-10 w-1 flex-shrink-0 rounded-full bg-gray-900" />
        <div data-tauri-drag-region className="min-w-0 flex-1">
          <div data-tauri-drag-region className="truncate text-sm font-semibold">
            {event.summary}
          </div>
          <div data-tauri-drag-region className="text-xs text-gray-500">
            {timeRange(event)}
          </div>
        </div>
        <div className="flex flex-shrink-0 overflow-hidden rounded-xl border border-gray-200">
          <button
            onClick={() => join(true)}
            className="flex items-center gap-2 bg-white px-3 py-2 text-left hover:bg-gray-50"
          >
            <Video className="h-5 w-5 text-gray-700" />
            <span className="leading-tight">
              <span className="block text-sm font-medium">Join {label}</span>
              <span className="block text-xs text-gray-500">& start notes</span>
            </span>
          </button>
          <button
            aria-label="More options"
            aria-expanded={menu}
            onClick={() => setMenu((m) => !m)}
            className="border-l border-gray-200 bg-gray-50 px-2.5 hover:bg-gray-100"
          >
            <ChevronDown className="h-4 w-4 text-gray-600" />
          </button>
        </div>
      </div>

      {menu && (
        <div className="absolute right-3 top-[84px] w-52 rounded-xl border border-gray-200 bg-white py-1 text-sm shadow-lg">
          <button className="block w-full px-4 py-2 text-left hover:bg-gray-50" onClick={() => join(false)}>
            Join meeting only
          </button>
          <button
            className="block w-full px-4 py-2 text-left hover:bg-gray-50"
            onClick={() => invoke('meeting_alert_open_graphite')}
          >
            Open Graphite
          </button>
          <div className="my-1 border-t border-gray-100" />
          <button
            className="block w-full px-4 py-2 text-left hover:bg-gray-50"
            onClick={async () => {
              await invoke('set_meeting_alert', { enabled: false });
            }}
          >
            Turn off these pop-ups
          </button>
        </div>
      )}
    </div>
  );
}

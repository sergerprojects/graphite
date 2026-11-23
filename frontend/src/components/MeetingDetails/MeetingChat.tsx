'use client';

import { useCallback, useEffect, useState } from 'react';
import { LiveChat } from '@/components/Live/LiveChat';
import { storageService } from '@/services/storageService';
import { readMeetingNotes } from '@/lib/capture';

interface Props {
  meetingId: string;
  title: string;
  folder: string | null | undefined;
}

const clock = (secs: number) => {
  const t = Math.max(0, Math.floor(secs));
  return `${String(Math.floor(t / 60)).padStart(2, '0')}:${String(t % 60).padStart(2, '0')}`;
};

/** Ask questions of a finished meeting. Reads the whole saved transcript each time, so fixes and renames count. */
export function MeetingChat({ meetingId, title, folder }: Props) {
  const [notes, setNotes] = useState('');

  useEffect(() => {
    setNotes('');
    if (!folder) return;
    let active = true;
    readMeetingNotes(folder)
      .then(t => active && setNotes(t.trim()))
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [folder]);

  const getTranscript = useCallback(async () => {
    const meeting = await storageService.getMeeting(meetingId);
    const rows = [...(meeting.transcripts ?? [])].sort((a, b) => (a.audio_start_time ?? 0) - (b.audio_start_time ?? 0));
    return rows
      .map(r => `[${clock(r.audio_start_time ?? 0)}] ${r.speaker ? `${r.speaker}: ` : ''}${r.text}`)
      .join('\n');
  }, [meetingId]);

  return <LiveChat finished getTranscript={getTranscript} notes={notes} title={title} attendees={[]} />;
}

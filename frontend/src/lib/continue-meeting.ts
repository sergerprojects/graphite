import { invoke } from '@tauri-apps/api/core';
import { startRecordingWithTitle } from '@/lib/calendar';

const KEY = 'graphiteContinueMeeting';
const MAX_AGE_MS = 12 * 60 * 60 * 1000;

/** Remember that the next recording that stops belongs to this meeting. */
export function rememberContinue(meetingId: string) {
  sessionStorage.setItem(KEY, JSON.stringify({ id: meetingId, at: Date.now() }));
}

/** Transcribe again and add the new transcript to this meeting when it stops. */
export function continueMeeting(meetingId: string, title: string, pathname: string | null, goHome: () => void) {
  rememberContinue(meetingId);
  startRecordingWithTitle(title, pathname, goHome);
}

/**
 * Called when a recording has just been saved as `newMeetingId`. If it was started with
 * "Continue recording", joins it onto the earlier meeting and returns that meeting's id.
 */
export async function finishContinuedRecording(newMeetingId: string): Promise<string | null> {
  const raw = sessionStorage.getItem(KEY);
  if (!raw) return null;
  sessionStorage.removeItem(KEY);
  try {
    const { id, at } = JSON.parse(raw) as { id: string; at: number };
    if (!id || id === newMeetingId || Date.now() - at > MAX_AGE_MS) return null;
    await invoke('meetings_merge', { targetId: id, sourceId: newMeetingId });
    return id;
  } catch (e) {
    console.error('Could not add the recording to the earlier meeting:', e);
    return null;
  }
}

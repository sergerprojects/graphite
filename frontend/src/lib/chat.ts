import { invoke } from '@tauri-apps/api/core';

export type ChatModel = 'haiku' | 'sonnet' | 'opus';
export interface ChatTurn {
  role: 'user' | 'assistant';
  text: string;
}

/** Ask Claude (the user's own Claude Code login) about the meeting, live or finished. */
export async function askMeeting(args: {
  question: string;
  transcript: string;
  notes: string;
  title: string;
  attendees: string[];
  history: ChatTurn[];
  model: ChatModel;
  /** True once the meeting is over: answers look back at the whole call. */
  finished?: boolean;
}): Promise<string> {
  return invoke<string>('meeting_chat', args);
}

export async function openExternal(url: string): Promise<void> {
  try {
    await invoke('open_external', { url });
  } catch {
    // ignore: only https links are opened
  }
}

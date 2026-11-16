import { invoke } from '@tauri-apps/api/core';

export interface SlackTarget {
  id: string;
  name: string;
  kind: 'channel' | 'private' | 'person';
  detail: string;
}

export type SlackKind = 'short' | 'full' | 'followup';

export const SLACK_KINDS: { value: SlackKind; label: string }[] = [
  { value: 'short', label: 'Short post' },
  { value: 'full', label: 'Full summary' },
  { value: 'followup', label: 'Follow-up' },
];

/** Channels and people matching what was typed (through your Slack connector in Claude Code). */
export function findSlackTargets(query: string): Promise<SlackTarget[]> {
  return invoke<SlackTarget[]>('slack_find', { query });
}

/** Draft the message text for Slack. */
export function composeSlack(args: { kind: SlackKind; title: string; attendees: string[]; body: string }): Promise<string> {
  return invoke<string>('slack_compose', args);
}

/** Save the message as a Slack draft (send = false) or post it now (send = true). */
export function postToSlack(args: { channelId: string; message: string; send: boolean }): Promise<{ link: string | null; sent: boolean }> {
  return invoke('slack_post', args);
}

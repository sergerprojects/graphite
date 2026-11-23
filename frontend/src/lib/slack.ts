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

const searchCache = new Map<string, SlackTarget[]>();

/** Channels and people matching what was typed (through your Slack connector in Claude Code). Repeats are instant. */
export async function findSlackTargets(query: string): Promise<SlackTarget[]> {
  const key = query.trim().toLowerCase();
  const hit = searchCache.get(key);
  if (hit) return hit;
  const found = await invoke<SlackTarget[]>('slack_find', { query });
  if (found.length > 0) searchCache.set(key, found);
  return found;
}

const RECENT_KEY = 'graphite.slack.recent';

/** Places you shared to before, newest first. Kept on this computer only. */
export function recentSlackTargets(): SlackTarget[] {
  try {
    const raw = JSON.parse(localStorage.getItem(RECENT_KEY) || '[]');
    return Array.isArray(raw) ? raw.filter(t => t && t.id && t.name) : [];
  } catch {
    return [];
  }
}

export function rememberSlackTarget(t: SlackTarget): void {
  try {
    const next = [t, ...recentSlackTargets().filter(x => x.id !== t.id)].slice(0, 12);
    localStorage.setItem(RECENT_KEY, JSON.stringify(next));
  } catch {
    // storage unavailable: nothing to remember
  }
}

/** Recent places whose name or detail contains every word typed. */
export function matchRecentTargets(query: string): SlackTarget[] {
  const words = query.toLowerCase().split(/\s+/).map(w => w.replace(/^#/, '')).filter(Boolean);
  const all = recentSlackTargets();
  if (words.length === 0) return all;
  return all.filter(t => {
    const hay = `${t.name} ${t.detail}`.toLowerCase();
    return words.every(w => hay.includes(w));
  });
}

/** Draft the message text for Slack. */
export function composeSlack(args: { kind: SlackKind; title: string; attendees: string[]; body: string }): Promise<string> {
  return invoke<string>('slack_compose', args);
}

/** Save the message as a Slack draft (send = false) or post it now (send = true). */
export function postToSlack(args: { channelId: string; message: string; send: boolean }): Promise<{ link: string | null; sent: boolean }> {
  return invoke('slack_post', args);
}

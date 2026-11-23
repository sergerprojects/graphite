'use client';

import { useEffect, useRef, useState } from 'react';
import { Loader2, Search, Send, FileText } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { composeSlack, findSlackTargets, matchRecentTargets, postToSlack, rememberSlackTarget, SlackKind, SlackTarget, SLACK_KINDS } from '@/lib/slack';
import { openExternal } from '@/lib/chat';

interface Props {
  title: string;
  attendees?: string[];
  /** The notes to turn into a post (summary, or notes and transcript while live). */
  getBody: () => Promise<string>;
  /** Which kinds of post make sense here; the first is the starting one. */
  kinds?: SlackKind[];
  label?: string;
  /** Hand back the dialog trigger so it fits in any toolbar. */
  trigger?: (open: () => void) => React.ReactNode;
}

/**
 * Share a meeting to Slack: pick a channel or person, look over the message, then save it as a
 * Slack draft (the default) or send it. Goes through the Slack connector in your own Claude Code.
 */
export function ShareToSlack({ title, attendees = [], getBody, kinds = ['short', 'full', 'followup'], label = 'Slack', trigger }: Props) {
  const [open, setOpen] = useState(false);
  const [kind, setKind] = useState<SlackKind>(kinds[0]);
  const [query, setQuery] = useState('');
  const [searching, setSearching] = useState(false);
  const [target, setTarget] = useState<SlackTarget | null>(null);
  const [message, setMessage] = useState('');
  const [composing, setComposing] = useState(false);
  const [posting, setPosting] = useState(false);
  const [confirmSend, setConfirmSend] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const bodyRef = useRef('');
  const seq = useRef(0);

  const compose = async (k: SlackKind) => {
    const mine = ++seq.current;
    setComposing(true);
    setError(null);
    try {
      if (!bodyRef.current) bodyRef.current = await getBody();
      const text = await composeSlack({ kind: k, title, attendees, body: bodyRef.current });
      if (mine === seq.current) setMessage(text);
    } catch (e) {
      if (mine === seq.current) setError(String(e));
    } finally {
      if (mine === seq.current) setComposing(false);
    }
  };

  // Draft the message as soon as the dialog opens, and again when the kind changes.
  useEffect(() => {
    if (!open) return;
    bodyRef.current = '';
    setTarget(null);
    setQuery('');
    setConfirmSend(false);
    compose(kind);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const pickKind = (k: SlackKind) => {
    setKind(k);
    compose(k);
  };

  // Places you used before show up instantly. The full search through Slack takes several seconds, so it
  // waits for a pause in typing (or Enter) and a newer search replaces an older one.
  const searchSeq = useRef(0);
  const [remote, setRemote] = useState<SlackTarget[]>([]);
  const runSearch = async (q: string) => {
    const mine = ++searchSeq.current;
    setSearching(true);
    try {
      const found = await findSlackTargets(q);
      if (mine === searchSeq.current) {
        setRemote(found);
        setError(null);
      }
    } catch (e) {
      if (mine === searchSeq.current && String(e) !== 'cancelled') setError(String(e));
    } finally {
      if (mine === searchSeq.current) setSearching(false);
    }
  };

  useEffect(() => {
    searchSeq.current++;
    setRemote([]);
    setSearching(false);
    if (!open || query.trim().length < 2 || target) return;
    const t = setTimeout(() => void runSearch(query), 800);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, open, target]);

  const recent = open && !target ? matchRecentTargets(query) : [];
  const shown = [...recent, ...remote.filter(r => !recent.some(x => x.id === r.id))];

  const choose = (t: SlackTarget) => {
    rememberSlackTarget(t);
    setTarget(t);
  };

  const post = async (send: boolean) => {
    if (!target) return;
    setPosting(true);
    setError(null);
    try {
      const r = await postToSlack({ channelId: target.id, message, send });
      toast.success(send ? `Sent to ${target.name}` : `Saved as a draft for ${target.name}`, {
        action: r.link ? { label: 'Open in Slack', onClick: () => openExternal(r.link!) } : undefined,
      });
      setOpen(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setPosting(false);
      setConfirmSend(false);
    }
  };

  return (
    <>
      {trigger ? (
        trigger(() => setOpen(true))
      ) : (
        <Button variant="outline" size="sm" title="Share to Slack" onClick={() => setOpen(true)}>
          <Send />
          <span className="hidden @[40rem]:inline">{label}</span>
        </Button>
      )}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-xl">
          <DialogHeader>
            <DialogTitle>Share to Slack</DialogTitle>
            <DialogDescription>Pick where it goes, look over the message, then save it as a draft or send it.</DialogDescription>
          </DialogHeader>

          <div className="space-y-3">
            {target ? (
              <div className="flex items-center justify-between rounded-md border border-gray-200 px-3 py-2 text-sm">
                <span>
                  <span className="font-medium">{target.name}</span>
                  <span className="ml-2 text-gray-500">{target.detail}</span>
                </span>
                <button className="text-xs text-blue-600 hover:underline" onClick={() => { setTarget(null); setQuery(''); }}>
                  Change
                </button>
              </div>
            ) : (
              <div>
                <div className="flex items-center gap-2 rounded-md border border-gray-200 px-3 py-2">
                  <Search className="h-4 w-4 text-gray-400" />
                  <input
                    autoFocus
                    value={query}
                    onChange={e => setQuery(e.target.value)}
                    onKeyDown={e => {
                      if (e.key === 'Enter' && query.trim().length >= 2) void runSearch(query);
                    }}
                    placeholder="Channel or person, e.g. pace-leaders or Sam"
                    className="flex-1 bg-transparent text-sm outline-none"
                  />
                  {searching && <Loader2 className="h-4 w-4 animate-spin text-gray-400" />}
                </div>
                {query.trim().length === 0 && shown.length > 0 && (
                  <p className="mt-1 px-1 text-xs text-gray-500">Recent</p>
                )}
                {shown.length > 0 && (
                  <ul className="mt-1 max-h-44 overflow-y-auto rounded-md border border-gray-200 text-sm">
                    {shown.map(r => (
                      <li key={r.id}>
                        <button className="flex w-full items-center justify-between px-3 py-1.5 text-left hover:bg-gray-50" onClick={() => choose(r)}>
                          <span className="font-medium">{r.name}</span>
                          <span className="ml-3 truncate text-xs text-gray-500">{r.detail}</span>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
                {query.trim().length >= 2 && (
                  <p className="mt-1 px-1 text-xs text-gray-500">
                    {searching ? 'Searching Slack…' : remote.length === 0 && shown.length === 0 ? 'Nothing yet. Press Enter to search Slack.' : 'Press Enter to search all of Slack.'}
                  </p>
                )}
              </div>
            )}

            <div className="flex gap-1">
              {SLACK_KINDS.filter(k => kinds.includes(k.value)).map(k => (
                <button
                  key={k.value}
                  onClick={() => pickKind(k.value)}
                  className={`rounded-full border px-3 py-1 text-xs ${kind === k.value ? 'border-gray-900 bg-gray-900 text-white' : 'border-gray-200 text-gray-600 hover:bg-gray-50'}`}
                >
                  {k.label}
                </button>
              ))}
            </div>

            <div className="relative">
              <textarea
                value={composing ? '' : message}
                onChange={e => setMessage(e.target.value)}
                placeholder={composing ? 'Writing the message…' : ''}
                rows={10}
                className="w-full rounded-md border border-gray-200 p-3 text-sm outline-none focus:border-gray-400"
              />
              {composing && <Loader2 className="absolute right-3 top-3 h-4 w-4 animate-spin text-gray-400" />}
            </div>

            {error && <p className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>}

            <div className="flex items-center justify-between">
              <p className="text-xs text-gray-500">Goes through the Slack connector in your Claude Code.</p>
              <div className="flex gap-2">
                {confirmSend ? (
                  <>
                    <Button variant="outline" size="sm" onClick={() => setConfirmSend(false)} disabled={posting}>Cancel</Button>
                    <Button size="sm" className="bg-red-600 text-white hover:bg-red-700" onClick={() => post(true)} disabled={posting}>
                      {posting ? <Loader2 className="animate-spin" /> : null}
                      Send to {target?.name} now
                    </Button>
                  </>
                ) : (
                  <>
                    <Button variant="outline" size="sm" disabled={!target || !message.trim() || composing || posting} onClick={() => setConfirmSend(true)}>
                      <Send /> Send now
                    </Button>
                    <Button size="sm" disabled={!target || !message.trim() || composing || posting} onClick={() => post(false)}>
                      {posting ? <Loader2 className="animate-spin" /> : <FileText />} Save as Slack draft
                    </Button>
                  </>
                )}
              </div>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

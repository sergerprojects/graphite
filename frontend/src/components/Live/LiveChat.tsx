'use client';

import { useEffect, useRef, useState } from 'react';
import { ArrowUp, Maximize2, Minimize2, SquarePen, X } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import { createPortal } from 'react-dom';
import { askMeeting, ChatModel, ChatTurn } from '@/lib/chat';
import { SavedPrompt, loadPrompts } from '@/lib/prompts';

const MODELS: { value: ChatModel; label: string }[] = [
  { value: 'haiku', label: 'Fast' },
  { value: 'sonnet', label: 'Balanced' },
  { value: 'opus', label: 'Deep' },
];

interface Props {
  /** Latest transcript as plain text, with speaker names. */
  getTranscript: () => string;
  notes: string;
  title: string;
  attendees: string[];
}

/** Chat with the meeting while it happens. Answers come from your own Claude Code login. */
export function LiveChat({ getTranscript, notes, title, attendees }: Props) {
  const [turns, setTurns] = useState<ChatTurn[]>([]);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [model, setModel] = useState<ChatModel>('sonnet');
  const [open, setOpen] = useState(false);
  // Pop the chat out to the whole window, like a full chat page.
  const [expanded, setExpanded] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);
  const [prompts, setPrompts] = useState<SavedPrompt[]>([]);

  useEffect(() => {
    const load = () => setPrompts(loadPrompts());
    load();
    window.addEventListener('graphite-prompts-changed', load);
    return () => window.removeEventListener('graphite-prompts-changed', load);
  }, []);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' });
  }, [turns, busy]);

  useEffect(() => {
    if (!expanded) return;
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setExpanded(false);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [expanded]);

  const ask = async (question: string) => {
    const q = question.trim();
    if (!q || busy) return;
    setOpen(true);
    setError(null);
    setInput('');
    const history = turns;
    setTurns([...history, { role: 'user', text: q }]);
    setBusy(true);
    try {
      const answer = await askMeeting({ question: q, transcript: getTranscript(), notes, title, attendees, history, model });
      setTurns(t => [...t, { role: 'assistant', text: answer }]);
    } catch (e) {
      setError(typeof e === 'string' ? e : 'Something went wrong. Try again.');
    } finally {
      setBusy(false);
    }
  };

  const md =
    'text-gray-900 [&_p]:mb-3 [&_ul]:mb-3 [&_ul]:ml-5 [&_ul]:list-disc [&_ol]:mb-3 [&_ol]:ml-5 [&_ol]:list-decimal [&_li]:mb-1 [&_strong]:font-semibold [&_h1]:mb-2 [&_h1]:text-lg [&_h1]:font-semibold [&_h2]:mb-2 [&_h2]:font-semibold [&_h3]:mb-1 [&_h3]:font-semibold';

  const messages = (big: boolean) => (
    <div className={big ? 'space-y-6 text-[16px] leading-7' : 'space-y-3 text-[15px] leading-6'}>
      {turns.map((t, i) =>
        t.role === 'user' ? (
          big ? (
            <div key={i} className="flex justify-end">
              <div className="max-w-[80%] rounded-2xl bg-gray-200/70 px-4 py-2 text-gray-900">{t.text}</div>
            </div>
          ) : (
            <div key={i} className="text-gray-500">{t.text}</div>
          )
        ) : (
          <div key={i} className={md}>
            <ReactMarkdown>{t.text}</ReactMarkdown>
          </div>
        )
      )}
      {busy && <div className="animate-pulse text-gray-400">Thinking…</div>}
      {error && <div className="text-sm text-red-600">{error}</div>}
      <div ref={endRef} />
    </div>
  );

  const composer = (big: boolean) => (
    <div className={`rounded-2xl border border-gray-200 bg-white/95 p-2 shadow-lg backdrop-blur ${big ? 'p-3' : ''}`}>
      <div className="mb-1.5 flex flex-wrap gap-1.5">
        {prompts.map(q => (
          <button
            key={q.id}
            onClick={() => ask(q.question)}
            disabled={busy}
            className="rounded-full border border-gray-200 px-2.5 py-1 text-xs text-gray-700 hover:bg-gray-50 disabled:opacity-50"
          >
            {q.label}
          </button>
        ))}
      </div>
      <div className="flex items-center gap-2">
        <input
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && !e.shiftKey && ask(input)}
          placeholder={big ? 'Ask anything about this meeting' : 'Ask about this meeting…'}
          className="min-w-0 flex-1 bg-transparent px-2 py-1.5 text-[15px] text-gray-900 outline-none placeholder:text-gray-400"
          aria-label="Ask about this meeting"
          autoFocus={big}
        />
        <select
          value={model}
          onChange={e => setModel(e.target.value as ChatModel)}
          className="rounded-md bg-transparent px-1 py-1 text-xs text-gray-500 outline-none hover:bg-gray-100"
          aria-label="Answer speed"
        >
          {MODELS.map(m => (
            <option key={m.value} value={m.value}>
              {m.label}
            </option>
          ))}
        </select>
        {!big && (
          <button
            onClick={() => {
              setExpanded(true);
              setOpen(true);
            }}
            className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            aria-label="Open chat in full window"
            title="Open chat in full window"
          >
            <Maximize2 className="h-4 w-4" />
          </button>
        )}
        <button
          onClick={() => ask(input)}
          disabled={busy || !input.trim()}
          className="flex h-8 w-8 items-center justify-center rounded-full bg-gray-900 text-white disabled:opacity-30"
          aria-label="Send"
        >
          <ArrowUp className="h-4 w-4" />
        </button>
      </div>
    </div>
  );

  if (expanded && typeof document !== 'undefined') {
    // Portal: a transformed ancestor would otherwise pin "fixed" to its own box.
    return createPortal(
      <div className="fixed inset-0 z-50 flex flex-col bg-gray-50" role="dialog" aria-label="Chat">
        <div className="flex items-center justify-between border-b border-gray-200 px-4 py-2.5">
          <div className="flex min-w-0 items-center gap-3">
            <button
              onClick={() => setExpanded(false)}
              className="flex items-center gap-1.5 rounded-lg px-2 py-1.5 text-sm text-gray-700 hover:bg-gray-100"
              aria-label="Back to the meeting"
            >
              <Minimize2 className="h-4 w-4" /> Back to meeting
            </button>
            <span className="truncate rounded-full border border-gray-200 bg-white px-3 py-1 text-xs text-gray-700">
              {title?.trim() || 'This meeting'}
            </span>
          </div>
          <button
            onClick={() => {
              setTurns([]);
              setError(null);
            }}
            disabled={turns.length === 0 && !error}
            className="flex items-center gap-1.5 rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-40"
          >
            <SquarePen className="h-4 w-4" /> New chat
          </button>
        </div>
        <div className="flex-1 overflow-y-auto px-6 py-8">
          <div className="mx-auto max-w-3xl">
            {turns.length === 0 && !busy ? (
              <div className="pt-24 text-center text-gray-400">Ask anything about this meeting.</div>
            ) : (
              messages(true)
            )}
          </div>
        </div>
        <div className="px-6 pb-6">
          <div className="mx-auto max-w-3xl">{composer(true)}</div>
        </div>
      </div>,
      document.body
    );
  }

  return (
    <div className="relative min-w-0 flex-1">
      <div>
        {open && (turns.length > 0 || busy || error) && (
          <div className="absolute bottom-full left-0 right-0 mb-2 max-h-[45vh] overflow-y-auto rounded-2xl border border-gray-200 bg-white p-4 shadow-lg">
            <div className="mb-2 flex justify-end gap-1">
              <button
                className="rounded p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
                onClick={() => setExpanded(true)}
                aria-label="Open chat in full window"
                title="Open chat in full window"
              >
                <Maximize2 className="h-4 w-4" />
              </button>
              <button
                className="rounded p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
                onClick={() => setOpen(false)}
                aria-label="Hide chat"
              >
                <X className="h-4 w-4" />
              </button>
            </div>
            {messages(false)}
          </div>
        )}
        {composer(false)}
      </div>
    </div>
  );
}

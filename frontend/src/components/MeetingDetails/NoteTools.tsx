"use client";

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Replace, Undo2, X } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { buildRefinePrompt } from '@/lib/refine-notes';

interface Props {
  meetingId: string;
  /** The notes exactly as they are on screen now, including unsaved edits. */
  getMarkdown: () => Promise<string>;
  /** Write any unsaved edits to the database. */
  saveEdits: () => Promise<void>;
  /** Start a regeneration with this extra context. */
  onGenerate: (prompt: string) => Promise<void>;
  /** Re-read the notes, transcript and title after they changed behind the editor's back. */
  onReload: () => Promise<void>;
}

interface FixResult {
  transcript: number;
  summary: number;
  title: number;
}

/** Whether there is a previous version of the notes to go back to, and how to go back. */
export function useNoteBackup(
  meetingId: string,
  saveEdits: () => Promise<void>,
  onReload: () => Promise<void>,
) {
  const [hasBackup, setHasBackup] = useState(false);
  const [busy, setBusy] = useState(false);

  const refreshBackup = useCallback(async () => {
    try {
      setHasBackup(await invoke<boolean>('has_summary_backup', { meetingId }));
    } catch {
      setHasBackup(false);
    }
  }, [meetingId]);

  useEffect(() => {
    void refreshBackup();
  }, [refreshBackup]);

  const undo = useCallback(async () => {
    setBusy(true);
    try {
      await saveEdits();
      const ok = await invoke<boolean>('restore_summary_backup', { meetingId });
      if (!ok) toast.error('Nothing to undo.');
      await onReload();
      await refreshBackup();
    } catch (e) {
      toast.error('Could not undo', { description: String(e) });
    } finally {
      setBusy(false);
    }
  }, [meetingId, saveEdits, onReload, refreshBackup]);

  return { hasBackup, setHasBackup, busy, undo };
}

/** Shown only while there is something to go back to. */
export function UndoButton({ onUndo, busy }: { onUndo: () => void; busy: boolean }) {
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={onUndo}
      disabled={busy}
      title="Go back to the notes as they were before the last update or fix. Click again to switch back."
    >
      <Undo2 size={18} />
      <span className="hidden @[40rem]:inline">Undo</span>
    </Button>
  );
}

/** Ask for a rewrite of the notes that keeps the person's own edits. Opened from the Regenerate menu. */
export function RefineNotesDialog({
  open,
  onOpenChange,
  meetingId,
  getMarkdown,
  saveEdits,
  onGenerate,
  onBackedUp,
}: Pick<Props, 'meetingId' | 'getMarkdown' | 'saveEdits' | 'onGenerate'> & {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onBackedUp: () => void;
}) {
  const [instruction, setInstruction] = useState('');
  const [busy, setBusy] = useState(false);

  const run = async () => {
    setBusy(true);
    try {
      await saveEdits();
      const markdown = await getMarkdown();
      if (!markdown.trim()) {
        toast.error('There are no notes to update yet.');
        return;
      }
      await invoke('backup_summary', { meetingId });
      onBackedUp();
      onOpenChange(false);
      await onGenerate(buildRefinePrompt(markdown, instruction));
      setInstruction('');
    } catch (e) {
      toast.error('Could not start the update', { description: String(e) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md space-y-3" aria-describedby={undefined}>
        <DialogTitle>Update from my edits</DialogTitle>
        <p className="text-sm text-gray-700">
          Graphite rewrites these notes from the transcript and keeps your edits, including your spellings,
          everywhere they apply. You can undo it.
        </p>
        <Textarea
          value={instruction}
          onChange={e => setInstruction(e.target.value)}
          placeholder="Anything else? e.g. “Keep it to five bullets.”"
          rows={3}
        />
        <div className="flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button size="sm" onClick={() => void run()} disabled={busy}>
            Update notes
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function FixWord({
  meetingId,
  saveEdits,
  onReload,
  onBackedUp,
  onUndo,
}: Pick<Props, 'meetingId' | 'saveEdits' | 'onReload'> & { onBackedUp: () => void; onUndo: () => void }) {
  const [open, setOpen] = useState(false);
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [remember, setRemember] = useState(true);
  const [words, setWords] = useState<string[]>([]);
  const [newWord, setNewWord] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    invoke<string[]>('get_vocabulary').then(setWords).catch(() => setWords([]));
  }, [open]);

  const saveWords = async (next: string[]) => {
    try {
      setWords(await invoke<string[]>('set_vocabulary', { terms: next }));
    } catch (e) {
      toast.error('Could not save your words', { description: String(e) });
    }
  };

  const replace = async () => {
    setBusy(true);
    try {
      await saveEdits();
      const r = await invoke<FixResult>('fix_word_everywhere', { meetingId, from, to });
      onBackedUp();
      const total = r.transcript + r.summary + r.title;
      if (remember && to.trim()) await saveWords([...words, to.trim()]);
      toast.success(
        total === 0
          ? `Didn’t find “${from.trim()}”.`
          : `Changed ${total} place${total === 1 ? '' : 's'} to “${to.trim()}”.`,
        {
          description: total > 0 && remember ? 'Saved to your words for future notes.' : undefined,
          action: total > 0 ? { label: 'Undo', onClick: onUndo } : undefined,
        },
      );
      setFrom('');
      setTo('');
      await onReload();
    } catch (e) {
      toast.error('Could not fix the word', { description: String(e) });
    } finally {
      setBusy(false);
    }
  };

  const addWord = async () => {
    if (!newWord.trim()) return;
    await saveWords([...words, newWord]);
    setNewWord('');
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" title="Fix a misheard word everywhere in this meeting">
          <Replace size={18} />
          <span className="hidden @[40rem]:inline">Fix a word</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-96 space-y-4">
        <div className="space-y-2">
          <p className="text-sm font-medium text-gray-800">Fix a word everywhere</p>
          <p className="text-xs text-gray-600">Changes the transcript, the notes and the title of this meeting.</p>
          <div className="flex items-center gap-2">
            <Input value={from} onChange={e => setFrom(e.target.value)} placeholder="Jetmine" aria-label="Wrong word" />
            <span className="text-gray-400">→</span>
            <Input value={to} onChange={e => setTo(e.target.value)} placeholder="Jumpmind" aria-label="Right word" />
          </div>
          <label className="flex items-center gap-2 text-xs text-gray-700">
            <input type="checkbox" checked={remember} onChange={e => setRemember(e.target.checked)} />
            Remember the right spelling for future notes
          </label>
          <div className="flex justify-end">
            <Button size="sm" onClick={() => void replace()} disabled={busy || !from.trim() || !to.trim()}>
              Fix it
            </Button>
          </div>
        </div>

        <div className="space-y-2 border-t border-gray-200 pt-3">
          <p className="text-sm font-medium text-gray-800">My words</p>
          <p className="text-xs text-gray-600">Names Graphite should always spell this way in new summaries.</p>
          <div className="flex flex-wrap gap-1.5">
            {words.length === 0 && <span className="text-xs text-gray-400">None yet.</span>}
            {words.map(w => (
              <span key={w} className="inline-flex items-center gap-1 rounded-full bg-gray-100 px-2 py-0.5 text-xs text-gray-800">
                {w}
                <button
                  type="button"
                  aria-label={`Remove ${w}`}
                  className="text-gray-400 hover:text-gray-700"
                  onClick={() => void saveWords(words.filter(x => x !== w))}
                >
                  <X size={12} />
                </button>
              </span>
            ))}
          </div>
          <div className="flex gap-2">
            <Input
              value={newWord}
              onChange={e => setNewWord(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && void addWord()}
              placeholder="Add a name or term"
              aria-label="Add a word"
            />
            <Button variant="outline" size="sm" onClick={() => void addWord()} disabled={!newWord.trim()}>
              Add
            </Button>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}

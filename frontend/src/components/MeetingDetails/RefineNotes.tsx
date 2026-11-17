"use client";

import { useState } from 'react';
import { Wand2 } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { buildRefinePrompt } from '@/lib/refine-notes';

interface Props {
  /** The notes exactly as they are on screen now, including unsaved edits. */
  getMarkdown: () => Promise<string>;
  /** Starts a regeneration with this extra context. */
  onRefine: (prompt: string) => Promise<void>;
}

/** "Update from my edits": rewrite the notes from the transcript, keeping the person's corrections. */
export function RefineNotes({ getMarkdown, onRefine }: Props) {
  const [open, setOpen] = useState(false);
  const [instruction, setInstruction] = useState('');
  const [busy, setBusy] = useState(false);

  const run = async () => {
    setBusy(true);
    try {
      const markdown = await getMarkdown();
      if (!markdown.trim()) {
        toast.error('There are no notes to update yet.');
        return;
      }
      setOpen(false);
      await onRefine(buildRefinePrompt(markdown, instruction));
      setInstruction('');
    } finally {
      setBusy(false);
    }
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" title="Rewrite the notes using your edits">
          <Wand2 size={18} />
          <span className="hidden @[40rem]:inline">Update from my edits</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-96 space-y-3">
        <p className="text-sm text-gray-700">
          Graphite rewrites these notes from the transcript and keeps your edits, including your spellings,
          everywhere they apply.
        </p>
        <Textarea
          value={instruction}
          onChange={e => setInstruction(e.target.value)}
          placeholder="Anything else? e.g. “It’s Jumpmind, not Jetmine. Keep it to five bullets.”"
          rows={3}
        />
        <div className="flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button size="sm" onClick={() => void run()} disabled={busy}>
            Update notes
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
}

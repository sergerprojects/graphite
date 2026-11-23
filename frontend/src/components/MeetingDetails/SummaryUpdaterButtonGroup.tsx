"use client";

import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, Send, Check, Loader2 } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import Analytics from '@/lib/analytics';
import { ShareToSlack } from '@/components/Slack/ShareToSlack';

interface Props {
  onCopy: () => Promise<void>;
  slackTitle: string;
  getSlackBody: () => Promise<string>;
}

/** Getting the summary out of Graphite: copy it, or share it to Slack. */
export function SummaryShareButtons({ onCopy, slackTitle, getSlackBody }: Props) {
  return (
    <ButtonGroup>
      <Button
        variant="outline"
        size="sm"
        title="Copy the summary"
        onClick={() => {
          Analytics.trackButtonClick('copy_summary', 'meeting_details');
          void onCopy();
        }}
      >
        <Copy />
        <span className="hidden @[32rem]:inline">Copy</span>
      </Button>
      <ShareToSlack
        title={slackTitle}
        kinds={['short', 'full', 'followup']}
        getBody={getSlackBody}
        trigger={open => (
          <Button variant="outline" size="sm" title="Share to Slack as a draft" onClick={open}>
            <Send />
            <span className="hidden @[32rem]:inline">Slack</span>
          </Button>
        )}
      />
    </ButtonGroup>
  );
}

/** Edits save themselves; this just says so. "Saving…" while there are unsaved edits, then "Saved" for a moment. */
export function SaveStatus({ dirty }: { dirty: boolean }) {
  const [justSaved, setJustSaved] = useState(false);
  const wasDirty = useRef(false);

  useEffect(() => {
    if (wasDirty.current && !dirty) {
      setJustSaved(true);
      const t = setTimeout(() => setJustSaved(false), 2500);
      wasDirty.current = false;
      return () => clearTimeout(t);
    }
    wasDirty.current = dirty;
    if (dirty) setJustSaved(false);
  }, [dirty]);

  if (dirty) {
    return (
      <span className="flex items-center gap-1 text-xs text-gray-500" aria-live="polite">
        <Loader2 size={12} className="animate-spin" /> Saving…
      </span>
    );
  }
  if (justSaved) {
    return (
      <span className="flex items-center gap-1 text-xs text-gray-500" aria-live="polite">
        <Check size={12} /> Saved
      </span>
    );
  }
  return null;
}

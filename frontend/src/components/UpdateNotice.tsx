'use client';

import { useEffect, useRef } from 'react';
import { toast } from 'sonner';
import { checkForUpdate } from '@/lib/beta';
import { findUpdate, installUpdate, restartGraphite } from '@/lib/updater';
import { useRecordingState } from '@/contexts/RecordingStateContext';

const KEY = 'graphiteUpdateNotice';
const EVERY = 4 * 60 * 60 * 1000;
const RETRY_WHILE_RECORDING = 10 * 60 * 1000;

/**
 * Keeps Graphite current. When a newer build is on GitHub it downloads and installs it quietly
 * (never during a recording), then offers a Restart. If the signed-update feed can't be read, it
 * falls back to telling the person once a day, with the line to paste. Silent when offline or up to date.
 */
export function UpdateNotice() {
  const { isRecording } = useRecordingState();
  const recording = useRef(isRecording);
  recording.current = isRecording;
  const installed = useRef<string | null>(null);

  useEffect(() => {
    let alive = true;
    let timer: ReturnType<typeof setTimeout>;

    const fallbackNotice = async () => {
      const today = new Date().toDateString();
      if (localStorage.getItem(KEY) === today) return;
      const u = await checkForUpdate();
      localStorage.setItem(KEY, today);
      if (!u.newer) return;
      toast(`Graphite ${u.latest} is available`, {
        description: 'Copy the update line, paste it in Terminal, and Graphite reopens updated.',
        duration: 20000,
        action: {
          label: 'Copy',
          onClick: () => navigator.clipboard.writeText(u.command).then(() => toast.success('Copied. Paste it in Terminal.')),
        },
      });
    };

    const tick = async () => {
      let next = EVERY;
      try {
        if (recording.current) {
          next = RETRY_WHILE_RECORDING; // never swap the app under a live meeting
        } else {
          const update = await findUpdate();
          if (update && installed.current !== update.version) {
            await installUpdate(update);
            installed.current = update.version;
            toast(`Graphite ${update.version} is installed`, {
              description: 'It starts the next time you open Graphite.',
              duration: 60000,
              action: { label: 'Restart now', onClick: () => void restartGraphite() },
            });
          }
        }
      } catch {
        try {
          await fallbackNotice();
        } catch {
          /* offline or no release: say nothing */
        }
      }
      if (alive) timer = setTimeout(tick, next);
    };

    timer = setTimeout(tick, 8000);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, []);
  return null;
}

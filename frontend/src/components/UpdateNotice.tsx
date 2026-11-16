'use client';

import { useEffect } from 'react';
import { toast } from 'sonner';
import { checkForUpdate } from '@/lib/beta';

const KEY = 'graphiteUpdateNotice';

/** Once a day, tell the person if a newer Graphite exists. Silent when offline or up to date. */
export function UpdateNotice() {
  useEffect(() => {
    const today = new Date().toDateString();
    if (localStorage.getItem(KEY) === today) return;
    const t = setTimeout(async () => {
      try {
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
      } catch {
        /* offline or no release: say nothing */
      }
    }, 8000);
    return () => clearTimeout(t);
  }, []);
  return null;
}

'use client';

import { useEffect, useState } from 'react';
import { CaptureStatus, getCaptureStatus, openPrivacyPane, requestAccessibility, requestScreenRecording } from '@/lib/capture';

/**
 * While transcribing: if call names or screen-share snapshots are switched on but macOS
 * has not allowed what they need, say so right here, with a button to fix it.
 */
export function CaptureNotice() {
  const [s, setS] = useState<CaptureStatus | null>(null);

  useEffect(() => {
    let alive = true;
    const poll = () => getCaptureStatus().then(v => alive && setS(v)).catch(() => {});
    poll();
    const id = setInterval(poll, 5000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  if (!s) return null;
  const needAccess = s.names && !s.accessibility;
  const needScreen = s.snapshots && !s.screen_recording;
  if (!needAccess && !needScreen) return null;

  const fix = async () => {
    if (needAccess) {
      await requestAccessibility().catch(() => {});
      await openPrivacyPane('Privacy_Accessibility').catch(() => {});
    } else {
      await requestScreenRecording().catch(() => {});
      await openPrivacyPane('Privacy_ScreenCapture').catch(() => {});
    }
  };

  return (
    <div className="mb-3 flex items-center justify-between gap-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
      <span>
        {needAccess && needScreen
          ? 'Speaker names and screen snapshots are off: macOS has not allowed Accessibility and Screen Recording for Graphite.'
          : needAccess
            ? 'Speaker names are off: macOS has not allowed Accessibility for Graphite.'
            : 'Screen snapshots are off: macOS has not allowed Screen Recording for Graphite.'}
      </span>
      <button onClick={fix} className="shrink-0 rounded-md bg-amber-900 px-2.5 py-1 text-xs text-white hover:bg-amber-800">
        Fix
      </button>
    </div>
  );
}

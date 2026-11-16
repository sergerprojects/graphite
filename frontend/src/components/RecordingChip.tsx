'use client';

import { usePathname, useRouter } from 'next/navigation';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { setLiveFocus, useLiveFocus } from '@/lib/liveView';

/**
 * Shown at the top while a meeting records and you are looking at something else
 * (Home, an old meeting, settings). One click returns to the live note.
 */
export function RecordingChip() {
  const { isRecording, isPaused } = useRecordingState();
  const { meetingTitle } = useTranscripts();
  const focused = useLiveFocus();
  const pathname = usePathname();
  const router = useRouter();

  const onLiveNote = pathname === '/' && focused;
  if (!isRecording || onLiveNote) return null;

  const open = () => {
    setLiveFocus(true);
    router.push('/');
  };

  return (
    <button
      onClick={open}
      aria-label="Return to the meeting being transcribed"
      title="Back to the live meeting"
      className="fixed left-1/2 top-2 z-40 flex max-w-[420px] -translate-x-1/2 items-center gap-2.5 rounded-full bg-gray-900 py-1.5 pl-3 pr-4 text-sm text-white shadow-lg hover:bg-gray-800"
    >
      <span className="flex h-4 items-center gap-[2px]" aria-hidden>
        {[0, 1, 2].map(i => (
          <span
            key={i}
            className={`w-[3px] rounded-full ${isPaused ? 'bg-amber-400' : 'animate-pulse bg-green-400'}`}
            style={{ height: isPaused ? 8 : [10, 16, 12][i], animationDelay: `${i * 150}ms` }}
          />
        ))}
      </span>
      <span className="truncate font-medium">{(typeof meetingTitle === 'string' ? meetingTitle.trim() : '') || 'Meeting'}</span>
      {isPaused && <span className="shrink-0 text-xs text-gray-300">Paused</span>}
    </button>
  );
}

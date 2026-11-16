"use client";

import { invoke } from '@tauri-apps/api/core';
import { rememberContinue } from '@/lib/continue-meeting';
import { useState, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, FolderOpen, Mic, RefreshCw } from 'lucide-react';
import { useRouter, usePathname } from 'next/navigation';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { getEventForMeeting, startRecordingForEvent, startRecordingWithTitle } from '@/lib/calendar';
import Analytics from '@/lib/analytics';
import { RetranscribeDialog } from './RetranscribeDialog';
import { useConfig } from '@/contexts/ConfigContext';


interface TranscriptButtonGroupProps {
  transcriptCount: number;
  onCopyTranscript: () => void;
  onOpenMeetingFolder: () => Promise<void>;
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;
}


export function TranscriptButtonGroup({
  transcriptCount,
  onCopyTranscript,
  onOpenMeetingFolder,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
}: TranscriptButtonGroupProps) {
  const { betaFeatures } = useConfig();
  const [showRetranscribeDialog, setShowRetranscribeDialog] = useState(false);
  const router = useRouter();
  const pathname = usePathname();
  const { currentMeeting, meetings } = useSidebar();

  // Start another recording for this meeting. The new transcript is joined onto it when it stops.
  // Use the calendar event only when it is this meeting's own: with two meetings booked at
  // once, the stored event can be the other one, and the title must stay the one you picked.
  const continueRecording = useCallback(async () => {
    if (meetingId) rememberContinue(meetingId);
    const title =
      meetings.find(m => m.id === meetingId)?.title ||
      (currentMeeting?.id === meetingId ? currentMeeting?.title ?? '' : '') ||
      'Continued meeting';
    const event = meetingId ? await getEventForMeeting(meetingId) : null;
    const goHome = () => router.push('/');
    const same = event && event.summary.trim().toLowerCase() === title.trim().toLowerCase();
    if (same) {
      await startRecordingForEvent(event, pathname, goHome);
    } else {
      await invoke('calendar_set_pending_event', { event: null }).catch(() => {});
      startRecordingWithTitle(title, pathname, goHome);
    }
  }, [meetingId, pathname, router, currentMeeting, meetings]);

  const handleRetranscribeComplete = useCallback(async () => {
    // Refetch transcripts to show the updated data
    if (onRefetchTranscripts) {
      await onRefetchTranscripts();
    }
  }, [onRefetchTranscripts]);

  return (
    <div className="flex items-center justify-center w-full gap-2">
      <ButtonGroup>
        <Button
          variant="outline"
          size="sm"
          className="px-2 @[22rem]:px-3"
          onClick={() => {
            Analytics.trackButtonClick('copy_transcript', 'meeting_details');
            onCopyTranscript();
          }}
          disabled={transcriptCount === 0}
          title={transcriptCount === 0 ? 'No transcript available' : 'Copy Transcript'}
        >
          <Copy />
          <span className="hidden @[22rem]:inline">Copy</span>
        </Button>

        <Button
          size="sm"
          variant="outline"
          className="px-2 @[22rem]:px-4"
          onClick={() => {
            Analytics.trackButtonClick('continue_recording', 'meeting_details');
            continueRecording();
          }}
          title="Keep transcribing this meeting"
        >
          <Mic className="@[22rem]:mr-2" size={16} />
          <span className="hidden @[22rem]:inline">Continue</span>
        </Button>

        <Button
          size="sm"
          variant="outline"
          className="px-2 @[22rem]:px-4"
          onClick={() => {
            Analytics.trackButtonClick('open_recording_folder', 'meeting_details');
            onOpenMeetingFolder();
          }}
          title="Open meeting folder"
        >
          <FolderOpen className="@[22rem]:mr-2" size={18} />
          <span className="hidden @[22rem]:inline">Folder</span>
        </Button>

        {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
          <Button
            size="sm"
            variant="outline"
            className="bg-gradient-to-r from-blue-50 to-purple-50 hover:from-blue-100 hover:to-purple-100 border-blue-200 px-2 @[22rem]:px-4"
            onClick={() => {
              Analytics.trackButtonClick('enhance_transcript', 'meeting_details');
              setShowRetranscribeDialog(true);
            }}
            title="Retranscribe to enhance your recorded audio"
          >
            <RefreshCw className="@[22rem]:mr-2" size={18} />
            <span className="hidden @[22rem]:inline">Enhance</span>
          </Button>
        )}
      </ButtonGroup>

      {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
        <RetranscribeDialog
          open={showRetranscribeDialog}
          onOpenChange={setShowRetranscribeDialog}
          meetingId={meetingId}
          meetingFolderPath={meetingFolderPath}
          onComplete={handleRetranscribeComplete}
        />
      )}
    </div>
  );
}

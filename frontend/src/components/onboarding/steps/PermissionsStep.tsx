import React, { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Mic, Volume2, UserRound, MonitorPlay } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { PermissionRow } from '../shared';
import { useOnboarding } from '@/contexts/OnboardingContext';
import {
  getCaptureStatus,
  requestAccessibility,
  requestScreenRecording,
  openPrivacyPane,
} from '@/lib/capture';
import type { PermissionStatus } from '@/types/onboarding';

export function PermissionsStep() {
  const { setPermissionStatus, setPermissionsSkipped, permissions, completeOnboarding } = useOnboarding();
  const [isPending, setIsPending] = useState(false);
  // Optional Google Meet extras. macOS has no prompt for these, so we open the
  // right Settings pane and watch for the switch to be turned on.
  const [accessibility, setAccessibility] = useState<PermissionStatus>('not_determined');
  const [screenRecording, setScreenRecording] = useState<PermissionStatus>('not_determined');

  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const st = await getCaptureStatus();
        if (!alive) return;
        setAccessibility(st.accessibility ? 'authorized' : 'not_determined');
        setScreenRecording(st.screen_recording ? 'authorized' : 'not_determined');
      } catch {
        /* status is only a nicety */
      }
    };
    poll();
    const id = setInterval(poll, 1500);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  const handleAccessibility = async () => {
    try {
      await requestAccessibility();
    } catch {
      /* fall through to Settings */
    }
    await openPrivacyPane('Privacy_Accessibility').catch(() => {});
  };

  const handleScreenRecording = async () => {
    try {
      await requestScreenRecording();
    } catch {
      /* fall through to Settings */
    }
    await openPrivacyPane('Privacy_ScreenCapture').catch(() => {});
  };

  // Check permissions - only logs current state, doesn't auto-authorize
  // Actual permission checks are done via explicit user actions (clicking Enable)
  const checkPermissions = useCallback(async () => {
    console.log('[PermissionsStep] Current permission states:');
    console.log(`  - Microphone: ${permissions.microphone}`);
    console.log(`  - System Audio: ${permissions.systemAudio}`);
    // Don't auto-set permissions based on device availability
    // Permissions should only be set after explicit user action via Enable button
  }, [permissions.microphone, permissions.systemAudio]);

  // Check permissions on mount
  useEffect(() => {
    checkPermissions();
  }, [checkPermissions]);

  // Request microphone permission
  const handleMicrophoneAction = async () => {
    if (permissions.microphone === 'denied') {
      // Try to open system settings
      try {
        await invoke('open_system_settings');
      } catch {
        alert('Please enable microphone access in System Preferences > Security & Privacy > Microphone');
      }
      return;
    }

    setIsPending(true);
    try {
      console.log('[PermissionsStep] Triggering microphone permission...');
      const granted = await invoke<boolean>('trigger_microphone_permission');
      console.log('[PermissionsStep] Microphone permission result:', granted);

      if (granted) {
        setPermissionStatus('microphone', 'authorized');
      } else {
        // Permission was denied or dialog was dismissed
        setPermissionStatus('microphone', 'denied');
      }
    } catch (err) {
      console.error('[PermissionsStep] Failed to request microphone permission:', err);
      setPermissionStatus('microphone', 'denied');
    } finally {
      setIsPending(false);
    }
  };

  // Request system audio permission
  const handleSystemAudioAction = async () => {
    if (permissions.systemAudio === 'denied') {
      // Try to open system settings
      try {
        await invoke('open_system_settings');
      } catch {
        alert('Please enable Audio Capture in System Settings → Privacy & Security → Audio Capture');
      }
      return;
    }

    setIsPending(true);
    try {
      console.log('[PermissionsStep] Triggering Audio Capture permission...');
      // Backend creates Core Audio tap, captures audio, and verifies it's not silence
      // Returns true if permission granted and audio verified, false if denied (silence)
      const granted = await invoke<boolean>('trigger_system_audio_permission_command');
      console.log('[PermissionsStep] System audio permission result:', granted);

      if (granted) {
        setPermissionStatus('systemAudio', 'authorized');
        console.log('[PermissionsStep] Audio Capture permission verified - audio is not silence');
      } else {
        // Permission was denied (audio is silence)
        setPermissionStatus('systemAudio', 'denied');
        console.log('[PermissionsStep] Audio Capture permission denied - audio is silence');
      }
    } catch (err) {
      console.error('[PermissionsStep] Failed to request system audio permission:', err);
      setPermissionStatus('systemAudio', 'denied');
    } finally {
      setIsPending(false);
    }
  };

  const handleFinish = async () => {
    try {
      await completeOnboarding();
      window.location.reload();
    } catch (error) {
      console.error('Failed to complete onboarding:', error);
    }
  };

  const handleSkip = async () => {
    setPermissionsSkipped(true);
    await handleFinish();
  };

  const allPermissionsGranted =
    permissions.microphone === 'authorized' &&
    permissions.systemAudio === 'authorized';

  return (
    <OnboardingContainer
      title="Allow access"
      description="macOS will ask. Click Allow each time. Graphite can't hear a call without these two."
      step={3}
      totalSteps={3}
    >
      <div className="max-w-lg mx-auto space-y-6">
        {/* Required */}
        <div className="space-y-4">
          <p className="text-sm font-medium text-gray-900">Required</p>
          <PermissionRow
            compact
            icon={<Mic className="w-5 h-5" />}
            title="Microphone"
            description="So Graphite can hear you. Click Enable, then Allow."
            status={permissions.microphone}
            isPending={isPending}
            onAction={handleMicrophoneAction}
          />

          <PermissionRow
            compact
            icon={<Volume2 className="w-5 h-5" />}
            title="Audio capture"
            description="So Graphite can hear the other people on the call. Click Enable, then Allow."
            status={permissions.systemAudio}
            isPending={isPending}
            onAction={handleSystemAudioAction}
          />
        </div>

        {/* Optional */}
        <div className="space-y-4">
          <div>
            <p className="text-sm font-medium text-gray-900">Optional, Google Meet only</p>
            <p className="text-xs text-gray-600 mt-1">
              Turn Graphite on in the System Settings page that opens. Skip these and recording still works.
            </p>
          </div>
          <PermissionRow
            compact
            icon={<UserRound className="w-5 h-5" />}
            title="Accessibility"
            description="Puts real names on the transcript instead of Speaker 1, 2, 3."
            status={accessibility}
            onAction={handleAccessibility}
          />
          <PermissionRow
            compact
            icon={<MonitorPlay className="w-5 h-5" />}
            title="Screen Recording"
            description="Saves pictures of slides people share. If macOS says to quit and reopen, do that."
            status={screenRecording}
            onAction={handleScreenRecording}
          />
        </div>

        {/* What else to know */}
        <div className="rounded-lg bg-gray-100 p-4 text-sm text-gray-700 space-y-1.5">
          <p className="font-medium text-gray-900">After setup</p>
          <p>Summaries and Chat use Claude Code on this Mac. Settings, General shows whether it is ready.</p>
          <p>Connect your calendar in Settings, Calendar to get meeting titles and a pop-up before each call.</p>
        </div>

        {/* Action Buttons */}
        <div className="flex flex-col gap-3 pt-2">
          <Button onClick={handleFinish} disabled={!allPermissionsGranted} className="w-full h-11">
            Finish setup
          </Button>

          <button
            onClick={handleSkip}
            className="text-sm text-neutral-500 hover:text-neutral-700 transition-colors"
          >
            I'll do this later
          </button>

          {!allPermissionsGranted ? (
            <p className="text-xs text-center text-muted-foreground">
              Recording won't work until both required items are allowed. Settings has them under Recording.
            </p>
          ) : (
            <p className="text-xs text-center text-muted-foreground">
              To record: press ⌘N or click the Graphite icon in the menu bar. Graphite also offers to record when a calendar meeting starts.
            </p>
          )}
        </div>
      </div>
    </OnboardingContainer>
  );
}

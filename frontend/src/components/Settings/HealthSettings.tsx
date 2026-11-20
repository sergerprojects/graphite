'use client';

import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { openExternal } from '@/lib/chat';
import {
  FEEDBACK_URL,
  UpdateInfo,
  appDiagnostics,
  checkForUpdate,
  claudeCodeStatus,
  getRememberVoices,
  installVoiceModels,
  openGraphiteFolder,
  setRememberVoices,
  voiceModelsStatus,
} from '@/lib/beta';
import { findUpdate, installUpdate, restartGraphite } from '@/lib/updater';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import type { Update } from '@tauri-apps/plugin-updater';

const row = 'flex items-center justify-between gap-4 rounded-lg border p-4';

function copy(text: string, done: string) {
  navigator.clipboard.writeText(text).then(
    () => toast.success(done),
    () => toast.error('Could not copy.')
  );
}

/** Is everything set up? Updates, Claude Code, voice models, diagnostics, where your data lives. */
export function HealthSettings() {
  const [claude, setClaude] = useState<boolean | null>(null);
  const [voice, setVoice] = useState<{ python_env: boolean; model: boolean } | null>(null);
  const [installing, setInstalling] = useState(false);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [updateMsg, setUpdateMsg] = useState('');
  const [checking, setChecking] = useState(false);
  const [remember, setRemember] = useState<boolean | null>(null);
  const { isRecording } = useRecordingState();
  const [signed, setSigned] = useState<Update | null>(null);
  const [updating, setUpdating] = useState(false);

  const refresh = useCallback(() => {
    claudeCodeStatus().then(s => setClaude(s.installed)).catch(() => setClaude(null));
    voiceModelsStatus().then(setVoice).catch(() => setVoice(null));
    getRememberVoices().then(setRemember).catch(() => setRemember(null));
  }, []);
  useEffect(refresh, [refresh]);

  const check = async () => {
    setChecking(true);
    setUpdateMsg('');
    try {
      const u = await checkForUpdate();
      setUpdate(u);
      setSigned(await findUpdate().catch(() => null));
      setUpdateMsg(u.newer ? `Version ${u.latest} is available.` : `You have the latest version (${u.current}).`);
    } catch (e) {
      setUpdate(null);
      setUpdateMsg(String(e));
    } finally {
      setChecking(false);
    }
  };

  const updateNow = async () => {
    if (!signed) return;
    setUpdating(true);
    try {
      await installUpdate(signed);
      toast.success(`Graphite ${signed.version} is installed. Restarting…`);
      await restartGraphite();
    } catch (e) {
      toast.error(`Could not update: ${String(e)}`, { duration: 12000 });
    } finally {
      setUpdating(false);
    }
  };

  const install = async () => {
    setInstalling(true);
    try {
      toast.success(await installVoiceModels(), { duration: 8000 });
    } catch (e) {
      toast.error(String(e), { duration: 12000 });
    } finally {
      setInstalling(false);
      refresh();
    }
  };

  const voiceReady = voice?.python_env && voice?.model;

  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold text-gray-900">Health and updates</h3>

      <div className={row}>
        <div className="flex-1">
          <div className="font-medium">Updates</div>
          <div className="text-sm text-gray-600">
            {updateMsg || 'Graphite checks for updates by itself and installs them between meetings. You can check now.'}
          </div>
          {update?.newer && !signed && (
            <pre className="mt-2 overflow-x-auto rounded bg-gray-100 p-2 text-xs">{update.command}</pre>
          )}
        </div>
        <div className="flex shrink-0 gap-2">
          {signed && (
            <Button size="sm" onClick={updateNow} disabled={updating || isRecording} title={isRecording ? 'Finish the meeting first' : undefined}>
              {updating ? 'Updating…' : 'Update and restart'}
            </Button>
          )}
          {update?.newer && !signed && (
            <>
              <Button size="sm" onClick={() => copy(update.command, 'Copied. Paste it in Terminal.')}>Copy command</Button>
              {update.url && <Button size="sm" variant="outline" onClick={() => openExternal(update.url)}>What&apos;s new</Button>}
            </>
          )}
          <Button size="sm" variant="outline" onClick={check} disabled={checking}>{checking ? 'Checking…' : 'Check now'}</Button>
        </div>
      </div>

      <div className={row}>
        <div className="flex-1">
          <div className="font-medium">Claude Code</div>
          <div className="text-sm text-gray-600">
            {claude === null
              ? 'Checking…'
              : claude
                ? 'Found. Summaries after each meeting and Chat use it. If they fail, open Terminal, run claude, and sign in.'
                : 'Not found. Summaries and Chat need it. Install it from claude.com/claude-code, then run claude in Terminal once to sign in.'}
          </div>
        </div>
        <span className={`text-sm ${claude ? 'text-green-700' : 'text-amber-700'}`}>{claude === null ? '' : claude ? 'Ready' : 'Missing'}</span>
      </div>

      <div className={row}>
        <div className="flex-1">
          <div className="font-medium">Voice models</div>
          <div className="text-sm text-gray-600">
            Tell people apart by voice (Speaker 1, 2, 3) when the call app can&apos;t name them. About 110 MB, installs in a few minutes.
          </div>
        </div>
        {voiceReady ? (
          <span className="text-sm text-green-700">Installed</span>
        ) : (
          <Button size="sm" onClick={install} disabled={installing}>{installing ? 'Installing…' : 'Install'}</Button>
        )}
      </div>

      <div className={row}>
        <div className="flex-1 pr-2">
          <div className="font-medium">Remember voices</div>
          <div className="text-sm text-gray-600">
            Saves the voices of people the call app named, on this Mac only, so the next meeting can name them sooner. These are voiceprints; some places require consent for them. Off by default.
          </div>
        </div>
        <Switch
          checked={remember === true}
          disabled={remember === null}
          onCheckedChange={v => {
            setRemember(v);
            setRememberVoices(v).catch(() => setRemember(!v));
          }}
        />
      </div>

      <div className={row}>
        <div className="flex-1">
          <div className="font-medium">Your data</div>
          <div className="text-sm text-gray-600">
            Transcripts and notes stay on this Mac, in your Movies folder (graphite-recordings) and Application Support. Delete a meeting from the sidebar; delete the folders to remove everything.
          </div>
        </div>
        <div className="flex shrink-0 gap-2">
          <Button size="sm" variant="outline" onClick={() => openGraphiteFolder('meetings')}>Meetings</Button>
          <Button size="sm" variant="outline" onClick={() => openGraphiteFolder('data')}>Settings folder</Button>
        </div>
      </div>

      <div className={row}>
        <div className="flex-1">
          <div className="font-medium">Something wrong?</div>
          <div className="text-sm text-gray-600">Copy a short report (versions and what is set up, no meeting content) and send it with your question.</div>
        </div>
        <div className="flex shrink-0 gap-2">
          <Button size="sm" variant="outline" onClick={async () => copy(await appDiagnostics(), 'Diagnostics copied.')}>Copy diagnostics</Button>
          <Button size="sm" variant="outline" onClick={() => openExternal(FEEDBACK_URL)}>Report a problem</Button>
        </div>
      </div>
    </div>
  );
}

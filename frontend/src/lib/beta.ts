import { invoke } from '@tauri-apps/api/core';

export interface UpdateInfo {
  current: string;
  latest: string;
  newer: boolean;
  url: string;
  notes: string;
  command: string;
}

/** Where people send problems. Change this one line if the project moves. */
export const FEEDBACK_URL = 'https://github.com/sergerprojects/graphite/issues/new';

export const claudeCodeStatus = () => invoke<{ installed: boolean; path: string | null }>('claude_code_status');
export const checkForUpdate = () => invoke<UpdateInfo>('check_for_update');
export const voiceModelsStatus = () => invoke<{ python_env: boolean; model: boolean }>('voice_models_status');
export const installVoiceModels = () => invoke<string>('install_voice_models');
export const appDiagnostics = () => invoke<string>('app_diagnostics');
export const openGraphiteFolder = (which: 'meetings' | 'data') => invoke<void>('open_graphite_folder', { which });
export const getRememberVoices = () => invoke<boolean>('get_remember_voices');
export const setRememberVoices = (enabled: boolean) => invoke<void>('set_remember_voices', { enabled });

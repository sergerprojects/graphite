import { invoke } from '@tauri-apps/api/core';

export interface Folder {
  id: string;
  name: string;
}

/** Window event fired after any folder change, so lists can refresh. */
export const FOLDERS_CHANGED = 'graphite-folders-changed';
/** A meeting being recorded has no saved id yet: its folder choice waits here. */
export const PENDING_FOLDER_KEY = 'graphitePendingFolder';

const changed = () => window.dispatchEvent(new Event(FOLDERS_CHANGED));

export const listFolders = () => invoke<Folder[]>('folders_list');
export const meetingFolders = () =>
  invoke<{ meeting_id: string; folder_id: string }[]>('meeting_folders');

export async function createFolder(name: string): Promise<Folder> {
  const f = await invoke<Folder>('folder_create', { name });
  changed();
  return f;
}
export async function renameFolder(id: string, name: string) {
  await invoke('folder_rename', { id, name });
  changed();
}
export async function deleteFolder(id: string) {
  await invoke('folder_delete', { id });
  changed();
}
export async function setMeetingFolder(meetingId: string, folderId: string | null) {
  await invoke('meeting_set_folder', { meetingId, folderId });
  changed();
}

/** After a recording is saved: file it in the folder picked while it ran. */
export async function applyPendingFolder(meetingId: string) {
  const pending = localStorage.getItem(PENDING_FOLDER_KEY);
  if (!pending) return;
  localStorage.removeItem(PENDING_FOLDER_KEY);
  await setMeetingFolder(meetingId, pending).catch(() => {});
}

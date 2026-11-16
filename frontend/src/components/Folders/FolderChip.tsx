'use client';

import { useEffect, useRef, useState } from 'react';
import { Check, FolderPlus, Folder as FolderIcon } from 'lucide-react';
import {
  Folder,
  FOLDERS_CHANGED,
  PENDING_FOLDER_KEY,
  createFolder,
  listFolders,
  meetingFolders,
  setMeetingFolder,
} from '@/lib/folders';

interface Props {
  /** The saved meeting's id; null while it is still recording (the choice waits for the save). */
  meetingId: string | null;
}

/** Chip that shows a meeting's folder and lets you file it, or make a new folder. */
export function FolderChip({ meetingId }: Props) {
  const [folders, setFolders] = useState<Folder[]>([]);
  const [current, setCurrent] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState('');
  const [error, setError] = useState('');
  const ref = useRef<HTMLDivElement>(null);

  const load = async () => {
    const list = await listFolders().catch(() => []);
    setFolders(list);
    if (meetingId) {
      const all = await meetingFolders().catch(() => []);
      setCurrent(all.find(m => m.meeting_id === meetingId)?.folder_id ?? null);
    } else {
      setCurrent(localStorage.getItem(PENDING_FOLDER_KEY));
    }
  };

  useEffect(() => {
    load();
    window.addEventListener(FOLDERS_CHANGED, load);
    return () => window.removeEventListener(FOLDERS_CHANGED, load);
  }, [meetingId]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [open]);

  const choose = async (id: string | null) => {
    if (meetingId) await setMeetingFolder(meetingId, id);
    else if (id) localStorage.setItem(PENDING_FOLDER_KEY, id);
    else localStorage.removeItem(PENDING_FOLDER_KEY);
    setCurrent(id);
    setOpen(false);
  };

  const makeFolder = async () => {
    setError('');
    try {
      const f = await createFolder(name);
      setName('');
      setNaming(false);
      await choose(f.id);
    } catch (e) {
      setError(typeof e === 'string' ? e : 'Could not create the folder.');
    }
  };

  const currentName = folders.find(f => f.id === current)?.name;

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen(o => !o)}
        aria-haspopup="menu"
        aria-expanded={open}
        title={currentName ? `In folder: ${currentName}` : 'Add to a folder'}
        className="flex h-7 items-center gap-1.5 rounded-full border border-gray-200 bg-white/70 px-2.5 text-xs text-gray-700 hover:bg-white"
      >
        {currentName ? <FolderIcon className="h-3.5 w-3.5" /> : <FolderPlus className="h-3.5 w-3.5" />}
        {currentName && <span className="max-w-[140px] truncate">{currentName}</span>}
      </button>

      {open && (
        <div role="menu" className="absolute left-0 top-9 z-30 w-60 rounded-xl border border-gray-200 bg-white py-1 text-sm shadow-lg">
          <button
            role="menuitem"
            onClick={() => choose(null)}
            className="flex w-full items-center justify-between px-3 py-2 text-left text-gray-600 hover:bg-gray-50"
          >
            No folder {current === null && <Check className="h-4 w-4" />}
          </button>
          {folders.map(f => (
            <button
              key={f.id}
              role="menuitem"
              onClick={() => choose(f.id)}
              className="flex w-full items-center justify-between px-3 py-2 text-left hover:bg-gray-50"
            >
              <span className="truncate">{f.name}</span>
              {current === f.id && <Check className="h-4 w-4" />}
            </button>
          ))}
          <div className="my-1 border-t border-gray-100" />
          {naming ? (
            <div className="px-3 py-2">
              <input
                autoFocus
                value={name}
                onChange={e => setName(e.target.value)}
                onKeyDown={e => {
                  if (e.key === 'Enter') makeFolder();
                  if (e.key === 'Escape') setNaming(false);
                }}
                placeholder="Folder name"
                className="w-full rounded-md border border-gray-300 px-2 py-1 text-sm outline-none focus:border-gray-500"
              />
              {error && <div className="mt-1 text-xs text-red-600">{error}</div>}
            </div>
          ) : (
            <button
              role="menuitem"
              onClick={() => setNaming(true)}
              className="flex w-full items-center gap-2 px-3 py-2 text-left hover:bg-gray-50"
            >
              <FolderPlus className="h-4 w-4" /> New folder…
            </button>
          )}
        </div>
      )}
    </div>
  );
}

'use client';

import { useEffect, useState } from 'react';
import { Plus } from 'lucide-react';
import { Folder, FOLDERS_CHANGED, createFolder, listFolders } from '@/lib/folders';

interface Props {
  active: string | null;
  onChange: (id: string | null) => void;
}

/** Row of folder chips above the meeting list: All, one per folder, and a "+" to add one. */
export function FolderBar({ active, onChange }: Props) {
  const [folders, setFolders] = useState<Folder[]>([]);
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState('');
  const [error, setError] = useState('');

  useEffect(() => {
    const load = () => listFolders().then(setFolders).catch(() => {});
    load();
    window.addEventListener(FOLDERS_CHANGED, load);
    return () => window.removeEventListener(FOLDERS_CHANGED, load);
  }, []);

  // A deleted folder can't stay selected.
  useEffect(() => {
    if (active && folders.length > 0 && !folders.some(f => f.id === active)) onChange(null);
  }, [folders, active, onChange]);

  const add = async () => {
    setError('');
    try {
      const f = await createFolder(name);
      setName('');
      setAdding(false);
      onChange(f.id);
    } catch (e) {
      setError(typeof e === 'string' ? e : 'Could not create the folder.');
    }
  };

  const chip = (on: boolean) =>
    `shrink-0 rounded-full border px-2.5 py-1 text-xs ${on ? 'border-gray-900 bg-gray-900 text-white' : 'border-gray-200 text-gray-700 hover:bg-gray-50'}`;

  return (
    <div className="px-3 pb-1 pt-2">
      <div className="custom-scrollbar flex items-center gap-1.5 overflow-x-auto pb-1" role="tablist" aria-label="Folders">
        <button role="tab" aria-selected={active === null} className={chip(active === null)} onClick={() => onChange(null)}>
          All
        </button>
        {folders.map(f => (
          <button key={f.id} role="tab" aria-selected={active === f.id} className={chip(active === f.id)} onClick={() => onChange(f.id)}>
            {f.name}
          </button>
        ))}
        {adding ? (
          <input
            autoFocus
            value={name}
            onChange={e => setName(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter') add();
              if (e.key === 'Escape') setAdding(false);
            }}
            onBlur={() => !name && setAdding(false)}
            placeholder="Folder name"
            className="w-28 shrink-0 rounded-full border border-gray-300 px-2.5 py-1 text-xs outline-none focus:border-gray-500"
          />
        ) : (
          <button aria-label="New folder" title="New folder" onClick={() => setAdding(true)} className="shrink-0 rounded-full border border-dashed border-gray-300 p-1 text-gray-500 hover:bg-gray-50">
            <Plus className="h-3.5 w-3.5" />
          </button>
        )}
      </div>
      {error && <div className="mt-1 text-xs text-red-600">{error}</div>}
    </div>
  );
}

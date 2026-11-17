'use client';

import { useCallback, useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { ChevronDown, ChevronUp, ScanLine, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { MeetingImage, deleteMeetingImage, listMeetingImages } from '@/lib/capture';
import { ImageViewer } from './ImageViewer';

interface Props {
  /** Meeting folder on disk. */
  folder: string | null | undefined;
  /** Re-check for new snapshots while a meeting is being recorded. */
  live?: boolean;
}

/** "Images N" stack under the notes. Click to expand, click a picture to enlarge. */
export function ImageStrip({ folder, live }: Props) {
  const [images, setImages] = useState<MeetingImage[]>([]);
  // While the meeting runs the pictures are always in view, like a filmstrip; on a saved meeting
  // they start folded.
  const [open, setOpen] = useState(!!live);
  const [viewing, setViewing] = useState<number | null>(null);
  const [confirmAll, setConfirmAll] = useState(false);

  const load = useCallback(() => {
    if (!folder) return;
    listMeetingImages(folder).then(setImages).catch(() => setImages([]));
  }, [folder]);

  useEffect(() => {
    load();
    if (!live) return;
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, [load, live]);

  const remove = async (name: string) => {
    if (!folder) return;
    try {
      await deleteMeetingImage(folder, name);
      setImages(prev => prev.filter(i => i.name !== name));
    } catch (e) {
      toast.error(`Could not remove the image: ${e}`);
    }
  };

  const removeAll = async () => {
    if (!folder) return;
    setConfirmAll(false);
    try {
      for (const im of images) await deleteMeetingImage(folder, im.name);
      setImages([]);
    } catch (e) {
      toast.error(`Could not remove all the images: ${e}`);
      load();
    }
  };

  if (!folder || images.length === 0) return null;

  return (
    <section className="mt-6" aria-label="Images from shared screens">
      {open && (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(112px,1fr))] gap-2.5">
          {live && (
            <div className="flex aspect-[16/10] flex-col items-center justify-center gap-1.5 rounded-lg border border-lime-200 bg-lime-50 text-xs text-gray-700">
              <ScanLine className="h-5 w-5 text-lime-600" aria-hidden />
              Taking snapshots
            </div>
          )}
          {images.map((im, i) => (
            <div key={im.name} className="group relative">
              <button
                className="block w-full overflow-hidden rounded-lg border border-gray-200 bg-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                onClick={() => setViewing(i)}
                aria-label={`Open image ${i + 1}`}
              >
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                  src={convertFileSrc(im.path)}
                  alt={im.text ? im.text.slice(0, 80) : `Shared screen ${i + 1}`}
                  loading="lazy"
                  className="aspect-[16/10] w-full object-cover transition group-hover:opacity-90"
                />
              </button>
              <button
                className="absolute right-1 top-1 rounded-full bg-white/90 p-1 text-gray-700 opacity-0 shadow transition hover:bg-white hover:text-red-600 focus:opacity-100 focus:outline-none focus:ring-2 focus:ring-red-500 group-hover:opacity-100"
                onClick={() => remove(im.name)}
                aria-label={`Delete image ${i + 1}`}
                title="Delete this image"
              >
                <Trash2 className="h-3.5 w-3.5" aria-hidden />
              </button>
            </div>
          ))}
        </div>
      )}

      <div className="mt-2 flex items-center justify-end gap-2">
        {open &&
          (confirmAll ? (
            <span className="flex items-center gap-2 text-sm text-gray-700">
              Delete all {images.length}?
              <button className="rounded-md px-2 py-1 text-red-600 hover:bg-red-50" onClick={removeAll}>
                Delete
              </button>
              <button className="rounded-md px-2 py-1 text-gray-600 hover:bg-gray-100" onClick={() => setConfirmAll(false)}>
                Keep
              </button>
            </span>
          ) : (
            <button
              className="rounded-md px-2 py-1 text-sm text-gray-500 hover:bg-gray-100 hover:text-red-600"
              onClick={() => setConfirmAll(true)}
            >
              Delete all
            </button>
          ))}
        <button
          className="flex items-center gap-1.5 rounded-md px-2 py-1 text-sm text-gray-600 hover:bg-gray-100"
          onClick={() => setOpen(o => !o)}
          aria-expanded={open}
        >
          {open ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
          Images
          <span className="rounded-md border border-gray-200 bg-white px-1.5 text-xs">{images.length}</span>
        </button>
      </div>

      {viewing !== null && (
        <ImageViewer
          folder={folder}
          images={images}
          index={Math.min(viewing, images.length - 1)}
          onIndex={setViewing}
          onClose={() => setViewing(null)}
          onDeleted={load}
        />
      )}
    </section>
  );
}

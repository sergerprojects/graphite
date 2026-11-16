'use client';

import { useEffect, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import { readMeetingNotes } from '@/lib/capture';

/** The notes the user typed during the meeting (notes.md), shown above the summary. */
export function MyNotes({ folder }: { folder: string | null | undefined }) {
  const [text, setText] = useState('');

  useEffect(() => {
    if (!folder) {
      setText('');
      return;
    }
    let active = true;
    readMeetingNotes(folder)
      .then(t => active && setText(t.trim()))
      .catch(() => active && setText(''));
    return () => {
      active = false;
    };
  }, [folder]);

  if (!text) return null;

  return (
    <section className="mb-6 rounded-lg border border-gray-200 bg-gray-50 p-4" aria-label="My notes">
      <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-500">My notes</h3>
      <div className="text-[15px] leading-7 text-gray-900 [&_h1]:mb-2 [&_h1]:text-xl [&_h1]:font-semibold [&_h2]:mb-1 [&_h2]:text-lg [&_h2]:font-semibold [&_h3]:font-semibold [&_ul]:ml-5 [&_ul]:list-disc [&_ol]:ml-5 [&_ol]:list-decimal [&_p]:mb-2 [&_strong]:font-semibold">
        <ReactMarkdown>{text}</ReactMarkdown>
      </div>
    </section>
  );
}

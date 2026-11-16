'use client';

import React, { useEffect, useRef } from 'react';
import { filterSuggestionItems } from '@blocknote/core';
import { getDefaultReactSlashMenuItems, SuggestionMenuController, useCreateBlockNote } from '@blocknote/react';
import { BlockNoteView } from '@blocknote/shadcn';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/shadcn/style.css';
import { NOTE_TEMPLATES } from '@/lib/notes-editing';

// Dead simple: the "/" menu offers only what you need in a meeting, plus the templates.
const KEEP = new Set(['Heading 1', 'Heading 2', 'Heading 3', 'Bullet List', 'Numbered List', 'Check List', 'Quote', 'Paragraph']);

interface Props {
  /** Markdown. Stored as notes.md, so the importer reads it unchanged. */
  value: string;
  onChange: (text: string) => void;
}

/**
 * Notes with live markdown: "# " makes a heading, "- " a bullet,
 * "1. " a numbered list, "[] " a checkbox, **bold** and *italic* render as you type,
 * and "/" opens a menu of blocks and meeting templates. The text is saved as markdown.
 */
export function NotesBox({ value, onChange }: Props) {
  const editor = useCreateBlockNote();
  // The markdown we last wrote out, so our own saves are not parsed back in.
  const lastEmitted = useRef<string | null>(null);
  const loaded = useRef(false);

  // Load (and reload when the text is replaced from outside, e.g. a restored draft).
  useEffect(() => {
    if (loaded.current && value === lastEmitted.current) return;
    loaded.current = true;
    let alive = true;
    (async () => {
      const blocks = value.trim() ? await editor.tryParseMarkdownToBlocks(value) : [];
      if (!alive) return;
      if (blocks.length > 0) editor.replaceBlocks(editor.document, blocks);
      lastEmitted.current = value;
    })();
    return () => {
      alive = false;
    };
  }, [value, editor]);

  const emit = async () => {
    const md = (await editor.blocksToMarkdownLossy(editor.document)).replace(/\n+$/, '');
    lastEmitted.current = md;
    onChange(md);
  };

  const templateItems = NOTE_TEMPLATES.map(t => ({
    title: t.label,
    subtext: 'Meeting template',
    group: 'Templates',
    aliases: [t.key, 'template'],
    onItemClick: async () => {
      const blocks = await editor.tryParseMarkdownToBlocks(t.body);
      const current = editor.getTextCursorPosition().block;
      editor.insertBlocks(blocks, current, 'after');
      const empty =
        Array.isArray(current.content) && current.content.every(c => c.type === 'text' && c.text.trim() === '');
      if (empty && current.type === 'paragraph') editor.removeBlocks([current]);
    },
  }));

  // Notion's block shortcuts: the editor already has bold, italic, underline, code, headings
  // (Cmd+Opt+1/2/3), numbered (Cmd+Shift+7) and bullets (Cmd+Shift+8). Add Notion's to-do
  // (Cmd+Shift+4) and toggle (Cmd+Shift+9).
  const onKeyDownCapture = (e: React.KeyboardEvent) => {
    if (!e.metaKey || !e.shiftKey || e.altKey || e.ctrlKey) return;
    const type = e.code === 'Digit4' ? 'checkListItem' : e.code === 'Digit9' ? 'toggleListItem' : null;
    if (!type) return;
    e.preventDefault();
    e.stopPropagation();
    editor.updateBlock(editor.getTextCursorPosition().block, { type });
  };

  return (
    <div className="notes-editor min-h-[55vh] text-[15px]" onKeyDownCapture={onKeyDownCapture}>
      <BlockNoteView
        editor={editor}
        theme="light"
        slashMenu={false}
        sideMenu={false}
        onChange={emit}
      >
        <SuggestionMenuController
          triggerCharacter="/"
          getItems={async query =>
            filterSuggestionItems(
              [...getDefaultReactSlashMenuItems(editor).filter(i => KEEP.has(i.title)), ...templateItems],
              query
            )
          }
        />
      </BlockNoteView>
    </div>
  );
}

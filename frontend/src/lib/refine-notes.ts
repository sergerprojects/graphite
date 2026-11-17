/**
 * The extra context for "update my notes from my edits": the notes as they are now (with the
 * person's corrections) plus anything they typed about what should change. The model rewrites
 * the notes from the transcript, treating the edited notes as the source of truth.
 */
export function buildRefinePrompt(editedMarkdown: string, instruction: string): string {
  const notes = editedMarkdown.trim();
  const ask = instruction.trim();
  const parts: string[] = [
    'The person has corrected the current notes below. Treat their wording as the source of truth: ' +
      'names, spellings, company and product names, facts and structure. Rewrite the final notes from the ' +
      'transcript so that every correction holds everywhere it applies, including places they did not edit. ' +
      'Where the transcript spells a word differently from the corrected notes, use the corrected spelling. ' +
      'Keep anything they added. Do not bring back anything they removed.',
  ];
  if (ask) parts.push(`What they asked for:\n${ask}`);
  if (notes) parts.push(`<corrected_notes>\n${notes}\n</corrected_notes>`);
  return parts.join('\n\n');
}

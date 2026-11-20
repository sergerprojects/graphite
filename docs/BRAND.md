# Graphite brand kit

Graphite is a notebook that happens to listen. The look is **Apple-clean with a paper feel**: quiet, lots of room, one accent. Everything below is the whole kit. If a choice isn't here, pick the plainer option.

## The mark
The app icon: a cream squircle, a slate pencil with a burgundy band and a pink eraser, one hand-drawn ink squiggle. Use the icon as is. Never add a gradient, glow or text to it.

The **squiggle** from the icon is the only decoration. Use it once per screen at most (under a page title, in an empty state). Not as a divider, not as a pattern.

## Colour
| Role | Name | Hex | Use |
|---|---|---|---|
| Page | Paper | `#f7f6f1` | the background of every screen |
| Sheet | White | `#ffffff` | cards, dialogs, anything that sits on the paper |
| Text | Ink | `#232a35` | text, primary buttons |
| Quiet text | Graphite | `#5d6068` | secondary text, captions |
| Lines | Hairline | `#e6e1d3` | borders and dividers (1px, never heavier) |
| Accent | Burgundy | `#86133a` | focus rings, hover on primary actions, small labels. The only accent |
| Highlight | Eraser | `#d97b93` | selected text (at 35%) and list bullets. Nowhere else |
| Signals | red / amber / green | tailwind defaults | recording, warnings, success only. Never decoration |

No blue, no purple, no gradients. In the app, the old blue and purple shades are mapped to slate ink, so nothing is blue by accident.

Dark mode inverts the same roles: night `#171b22`, sheet `#1f242d`, text `#f1ece0`, accent pink `#e48fa6`.

## Type
- **Interface:** the system sans (SF Pro on Mac, Segoe UI on Windows). 14 to 16px, regular and semibold. No other weights.
- **Titles and note headings:** the system serif (New York on Mac, Georgia on Windows). Meeting titles, "Coming up", headings inside notes. This is the notebook cue; it appears only on titles.
- **Time stamps and commands:** the system mono, tabular figures.

## Shape and space
- Radius: 10px for controls, 14px for sheets, full round for pills and chips.
- Borders are hairlines; shadows are rare (floating menus only, soft and low).
- Space on a 4px grid. When in doubt, add room rather than add a line.

## Motion
150ms ease-out. Nothing bounces, nothing loops except the recording pulse.

## Voice
Plain, short, friendly. Say what happened and what to do. No jargon, no exclamation marks.

## Where it lives
- App: `frontend/src/app/globals.css` (the tokens) and `frontend/tailwind.config.js` (paper greys, slate in place of blue and purple).
- Page: `docs/index.html`.

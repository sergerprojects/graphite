# Graphite interface

The Next.js (static export) interface that runs inside the Tauri app in `src-tauri/`. See the [project README](../README.md) and [docs/BUILDING.md](../docs/BUILDING.md).

```bash
npx pnpm@10 install
npx pnpm@10 run dev      # interface only, http://localhost:3118 (set NEXT_PUBLIC_MOCK_TAURI=1 to stub the app backend)
npx tsc --noEmit         # typecheck
```

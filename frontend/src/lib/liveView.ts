import { useSyncExternalStore } from 'react';

/**
 * While a meeting records, "/" shows the live note. You can still go Home (or anywhere
 * else): `focused` says whether the live note is the thing you are looking at.
 * Starting a recording focuses it; the Home button un-focuses it; the recording chip
 * focuses it again.
 */
let focused = true;
const listeners = new Set<() => void>();

export const setLiveFocus = (value: boolean) => {
  if (focused === value) return;
  focused = value;
  listeners.forEach(l => l());
};

export const getLiveFocus = () => focused;

export function useLiveFocus(): boolean {
  return useSyncExternalStore(
    cb => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    () => focused,
    () => true
  );
}

import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

/** A newer signed build on GitHub, or null. Throws when the update feed can't be read. */
export const findUpdate = (): Promise<Update | null> => check();

/** Download the build and swap it in. The running app keeps going until it restarts. */
export async function installUpdate(update: Update): Promise<void> {
  await update.downloadAndInstall();
}

export const restartGraphite = () => relaunch();

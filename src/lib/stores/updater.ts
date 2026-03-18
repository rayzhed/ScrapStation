import { writable } from 'svelte/store';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type UpdatePhase = 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error';

export interface UpdateState {
    phase: UpdatePhase;
    version?: string;       // available version, e.g. "0.1.1"
    notes?: string;         // release notes
    progress?: number;      // 0-100 during download
    error?: string;
}

export const updateState = writable<UpdateState>({ phase: 'idle' });

let _pending: Update | null = null;

export async function checkForUpdates(): Promise<void> {
    updateState.set({ phase: 'checking' });
    try {
        const update = await check();
        if (update) {
            _pending = update;
            updateState.set({
                phase: 'available',
                version: update.version,
                notes: update.body ?? undefined,
            });
        } else {
            updateState.set({ phase: 'idle' });
        }
    } catch (e) {
        updateState.set({ phase: 'error', error: String(e) });
    }
}

export async function installUpdate(): Promise<void> {
    if (!_pending) return;

    let downloaded = 0;
    let total = 0;

    updateState.update(s => ({ ...s, phase: 'downloading', progress: 0 }));

    try {
        await _pending.downloadAndInstall((event) => {
            if (event.event === 'Started') {
                total = event.data.contentLength ?? 0;
            } else if (event.event === 'Progress') {
                downloaded += event.data.chunkLength;
                const pct = total > 0 ? Math.round((downloaded / total) * 100) : 0;
                updateState.update(s => ({ ...s, progress: pct }));
            } else if (event.event === 'Finished') {
                updateState.update(s => ({ ...s, phase: 'ready', progress: 100 }));
            }
        });
    } catch (e) {
        updateState.set({ phase: 'error', error: String(e) });
    }
}

export async function applyUpdate(): Promise<void> {
    await relaunch();
}

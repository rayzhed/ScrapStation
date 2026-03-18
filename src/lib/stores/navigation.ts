import { writable } from 'svelte/store';

export type AppMode = 'updates' | 'browse' | 'library' | 'downloads';

/** Global navigation mode — any component can read or set this. */
export const currentMode = writable<AppMode>('browse');

export function navigateTo(mode: AppMode): void {
    currentMode.set(mode);
}

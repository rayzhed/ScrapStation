import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Source } from '$lib/types';
import { loadGames } from '$lib/stores/games';

interface SourceMetadata {
    id: string;
    name: string;
    description?: string;
    base_url: string;
    icon: string;
    color: string;
    search_debounce_ms: number;
    search_min_chars: number;
}

export const sourcesMetadata = writable<SourceMetadata[]>([]);
export const currentSource = writable<string>('');

export const sources = derived(sourcesMetadata, ($metadata) =>
    $metadata.map(meta => ({
        id: meta.id,
        name: meta.name,
        description: meta.description,
        icon: meta.icon,
        color: meta.color,
    } as Source))
);

/** Debounce delay (ms) for the currently selected source's search input. */
export const searchDebounceMs = derived(
    [sourcesMetadata, currentSource],
    ([$metadata, $current]) =>
        $metadata.find(m => m.id === $current)?.search_debounce_ms ?? 300
);

/** Minimum characters required before search fires for the current source. */
export const searchMinChars = derived(
    [sourcesMetadata, currentSource],
    ([$metadata, $current]) =>
        $metadata.find(m => m.id === $current)?.search_min_chars ?? 1
);

export async function loadAvailableSources() {
    try {
        const sourceIds = await invoke<string[]>('list_sources');

        const metadata = await Promise.all(
            sourceIds.map(id => invoke<SourceMetadata>('get_source_metadata', { sourceId: id }))
        );

        sourcesMetadata.set(metadata);

        if (sourceIds.length > 0) {
            currentSource.set(sourceIds[0]);
            await loadGames(sourceIds[0], 1);
        }
    } catch (error) {
        console.error('[sources] Failed to load sources:', error);
    }
}

/** Reload sources without resetting the current selection or games. */
async function reloadSourcesList() {
    try {
        const sourceIds = await invoke<string[]>('list_sources');
        const metadata = await Promise.all(
            sourceIds.map(id => invoke<SourceMetadata>('get_source_metadata', { sourceId: id }))
        );
        sourcesMetadata.set(metadata);
        // If selected source was removed, fall back to first available
        let current = '';
        const unsub = currentSource.subscribe(v => { current = v; });
        unsub();
        if (current && !sourceIds.includes(current) && sourceIds.length > 0) {
            currentSource.set(sourceIds[0]);
            await loadGames(sourceIds[0], 1);
        }
    } catch (error) {
        console.error('[sources] Failed to reload sources:', error);
    }
}

/** Start listening for backend source-file changes. Call once at app startup. */
export async function initSourceWatcher() {
    await listen('sources-changed', () => {
        reloadSourcesList().catch(e => console.error('[sources] Watcher reload failed:', e));
    });
}

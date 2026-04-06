import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export interface StorageConfig {
    data_root: string | null;
    effective_download_path: string;
    effective_library_path: string;
    default_download_path: string;
    default_library_path: string;
}

export const storageConfig = writable<StorageConfig | null>(null);

export async function loadStorageConfig(): Promise<void> {
    try {
        const config = await invoke<StorageConfig>('get_storage_config');
        storageConfig.set(config);
    } catch (e) {
        console.error('[appSettings] Failed to load storage config:', e);
    }
}

export async function setDataRoot(root: string | null): Promise<void> {
    await invoke('set_data_root', { root });
    await loadStorageConfig();
}

export interface KnownLibraryLocation {
    path: string;
    label: string;
    is_current: boolean;
    removable: boolean;
}

export async function getKnownLibraryLocations(): Promise<KnownLibraryLocation[]> {
    return invoke<KnownLibraryLocation[]>('get_known_library_locations');
}

export async function addLibraryLocation(path: string): Promise<void> {
    await invoke('add_library_location', { path });
}

export async function removeLibraryLocation(path: string): Promise<void> {
    await invoke('remove_library_location', { path });
}

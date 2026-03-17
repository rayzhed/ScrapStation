import { writable, derived, get } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { LibraryGame, ExtractionProgress, GameExecutable } from '$lib/types';
import { downloads } from './downloads';

// Backend format (snake_case)
interface LibraryGameBackend {
    id: string;
    source_slug: string;
    source_game_id: string;
    title: string;
    cover_url?: string;
    cover_path?: string;
    install_path: string;
    install_size: number;
    status: 'downloading' | 'extracting' | 'ready' | 'corrupted';
    installed_at: number;
    last_played?: number;
    total_playtime: number;
    executables: GameExecutable[];
    primary_exe?: string;
    archive_password?: string;
    download_ids: string[];
}

// Convert backend format to frontend format
function toFrontendFormat(g: LibraryGameBackend): LibraryGame {
    return {
        id: g.id,
        source_slug: g.source_slug,
        source_game_id: g.source_game_id,
        title: g.title,
        cover_url: g.cover_url,
        cover_path: g.cover_path,
        install_path: g.install_path,
        install_size: g.install_size,
        status: g.status,
        installed_at: g.installed_at,
        last_played: g.last_played,
        total_playtime: g.total_playtime,
        executables: g.executables,
        primary_exe: g.primary_exe,
        archive_password: g.archive_password,
        download_ids: g.download_ids,
    };
}

// ========== STORES ==========

export const libraryGames = writable<LibraryGame[]>([]);
export const extractionProgress = writable<Map<string, ExtractionProgress>>(new Map());

// Derived: Only ready games
export const readyGames = derived(libraryGames, $games =>
    $games.filter(g => g.status === 'ready')
);

// Derived: Games being installed (downloading or extracting)
export const installingGames = derived(libraryGames, $games =>
    $games.filter(g => g.status === 'downloading' || g.status === 'extracting')
);

// Derived: Corrupted games
export const corruptedGames = derived(libraryGames, $games =>
    $games.filter(g => g.status === 'corrupted')
);

// Derived: Library statistics
export const libraryStats = derived(libraryGames, $games => {
    const ready = $games.filter(g => g.status === 'ready');
    const totalSize = ready.reduce((sum, g) => sum + g.install_size, 0);
    const totalPlaytime = ready.reduce((sum, g) => sum + g.total_playtime, 0);

    return {
        gameCount: ready.length,
        totalSize,
        totalPlaytime,
        installingCount: $games.filter(g => g.status === 'downloading' || g.status === 'extracting').length,
    };
});

// ========== API FUNCTIONS ==========

// Load library from backend
export async function loadLibrary(): Promise<void> {
    try {
        const data = await invoke<LibraryGameBackend[]>('get_library_games');
        libraryGames.set(data.map(toFrontendFormat));
    } catch (e) {
    }
}

// Get a specific game
export async function getLibraryGame(id: string): Promise<LibraryGame | null> {
    try {
        const data = await invoke<LibraryGameBackend | null>('get_library_game', { id });
        return data ? toFrontendFormat(data) : null;
    } catch (e) {
        return null;
    }
}

// Add a game to library (called when download starts)
export async function addGameToLibrary(params: {
    sourceSlug: string;
    sourceGameId: string;
    title: string;
    coverUrl?: string;
    archivePassword?: string;
}): Promise<string> {
    const id = await invoke<string>('add_game_to_library', {
        sourceSlug: params.sourceSlug,
        sourceGameId: params.sourceGameId,
        title: params.title,
        coverUrl: params.coverUrl || null,
        archivePassword: params.archivePassword || null,
    });
    // Reload library to get updated state
    await loadLibrary();
    return id;
}

// Remove a game from library
export async function removeFromLibrary(id: string, deleteFiles: boolean = false): Promise<void> {
    try {
        await invoke('remove_from_library', { id, deleteFiles });
        libraryGames.update(games => games.filter(g => g.id !== id));
    } catch (e) {
        throw e;
    }
}

// Link a download ID to a library game
export async function linkDownloadToLibrary(gameId: string, downloadId: string): Promise<void> {
    try {
        await invoke('link_download_to_library', { gameId, downloadId });
        // Reload library to get updated download_ids
        await loadLibrary();
    } catch (e) {
    }
}

// Check if game is in library
export async function isGameInLibrary(sourceSlug: string, sourceGameId: string): Promise<LibraryGame | null> {
    try {
        const data = await invoke<LibraryGameBackend | null>('is_game_in_library', {
            sourceSlug,
            sourceGameId,
        });
        return data ? toFrontendFormat(data) : null;
    } catch (e) {
        return null;
    }
}

// Update archive password for a library game
export async function updateArchivePassword(gameId: string, password: string | undefined): Promise<void> {
    try {
        await invoke('update_archive_password', {
            gameId,
            password: password || null,
        });
    } catch (e) {
    }
}

// Get library game ID for a source game
export function getLibraryGameId(sourceSlug: string, sourceGameId: string): Promise<string> {
    return invoke<string>('get_library_game_id', { sourceSlug, sourceGameId });
}

// ========== EXTRACTION ==========

// Extract archives to library
export async function extractToLibrary(
    gameId: string,
    archivePaths: string[],
    password?: string
): Promise<void> {
    try {
        await invoke('extract_to_library', {
            gameId,
            archivePaths,
            password: password || null,
        });
    } catch (e) {
        throw e;
    }
}

// ========== GAME LAUNCH ==========

// Launch a game
export async function launchGame(id: string): Promise<void> {
    try {
        await invoke('launch_game', { id });
    } catch (e) {
        throw e;
    }
}

// Set primary executable
export async function setGameExecutable(id: string, executablePath: string): Promise<void> {
    try {
        await invoke('set_game_executable', { id, executablePath });
        libraryGames.update(games =>
            games.map(g => g.id === id ? { ...g, primary_exe: executablePath } : g)
        );
    } catch (e) {
        throw e;
    }
}

// Rescan executables for a game
export async function rescanGameExecutables(id: string): Promise<GameExecutable[]> {
    try {
        const executables = await invoke<GameExecutable[]>('rescan_game_executables', { id });
        libraryGames.update(games =>
            games.map(g => g.id === id ? { ...g, executables } : g)
        );
        return executables;
    } catch (e) {
        throw e;
    }
}

// ========== UTILITY ==========

// Open game folder in file explorer
export async function openGameFolder(id: string): Promise<void> {
    try {
        await invoke('open_game_folder', { id });
    } catch (e) {
        throw e;
    }
}

// Get library folder path
export async function getLibraryFolderPath(): Promise<string> {
    return invoke<string>('get_library_folder_path');
}

// Format playtime
export function formatPlaytime(seconds: number): string {
    if (seconds < 60) return 'Less than a minute';
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (hours > 0) {
        return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
}

// Format install size
export function formatSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// ========== AUTO-EXTRACTION ==========

// Track games currently being auto-extracted to avoid duplicates
const autoExtractingGames = new Set<string>();

// Check if all downloads for a library game are complete
function areAllDownloadsComplete(game: LibraryGame): boolean {
    if (!game.download_ids || game.download_ids.length === 0) {
        return false;
    }

    const currentDownloads = get(downloads);

    for (const dlId of game.download_ids) {
        const dl = currentDownloads.find(d => d.id === dlId);
        if (!dl || dl.status !== 'completed') {
            return false;
        }
    }
    return true;
}

// Get archive paths for a library game from completed downloads
function getArchivePathsForGame(game: LibraryGame): string[] {
    const currentDownloads = get(downloads);
    const paths: string[] = [];

    for (const dlId of game.download_ids) {
        const dl = currentDownloads.find(d => d.id === dlId);
        if (dl?.status === 'completed' && dl.filePath) {
            // Only include archive files
            if (/\.(rar|7z|zip)$/i.test(dl.filePath)) {
                paths.push(dl.filePath);
            }
        }
    }
    return paths;
}

// Try to auto-extract a game if all downloads are complete
async function tryAutoExtract(game: LibraryGame): Promise<void> {
    // Skip if already extracting
    if (autoExtractingGames.has(game.id)) {
        return;
    }
    if (game.status !== 'downloading') {
        return;
    }

    // Check if all downloads are complete
    const allComplete = areAllDownloadsComplete(game);
    if (!allComplete) return;

    const archivePaths = getArchivePathsForGame(game);

    if (archivePaths.length === 0) {
        return;
    }

    autoExtractingGames.add(game.id);

    try {
        await extractToLibrary(game.id, archivePaths, game.archive_password);
    } catch (e) {
    } finally {
        autoExtractingGames.delete(game.id);
    }
}

// Check all downloading games for auto-extraction eligibility
async function checkAutoExtraction(): Promise<void> {
    const games = get(libraryGames);
    const downloadingGames = games.filter(g => g.status === 'downloading');

    for (const game of downloadingGames) {
        await tryAutoExtract(game);
    }
}

// ========== COVER HELPERS ==========

/**
 * Returns the best available image source for a library game.
 * Prefers the locally-cached cover_path (offline-ready) over the remote cover_url.
 */
export function getBestCoverSource(game: LibraryGame): string {
    return game.cover_path ?? game.cover_url ?? '';
}

// ========== EVENT LISTENERS ==========

export async function initLibraryEvents(): Promise<void> {
    // Load library first
    await loadLibrary();

    // Cache covers for any game that has a URL but no local file yet
    invoke('cache_missing_covers').catch(() => {});

    // Listen for library updates from backend
    await listen<LibraryGameBackend[]>('library-updated', (event) => {
        libraryGames.set(event.payload.map(toFrontendFormat));
    });

    // Listen for download completion to trigger auto-extraction
    await listen<{ id: string; file_path: string }>('download-complete', async (_event) => {
        // Small delay to ensure download store is updated
        setTimeout(async () => {
            await checkAutoExtraction();
        }, 500);
    });

    // Listen for extraction progress
    await listen<ExtractionProgress>('extraction-progress', (event) => {
        extractionProgress.update(map => {
            const newMap = new Map(map);
            newMap.set(event.payload.game_id, event.payload);
            return newMap;
        });
    });

    // Listen for extraction complete
    await listen<{ gameId: string; success: boolean }>('extraction-complete', (event) => {
        const { gameId, success } = event.payload;

        // Remove from progress tracking
        extractionProgress.update(map => {
            const newMap = new Map(map);
            newMap.delete(gameId);
            return newMap;
        });

        // Reload library to get updated state
        loadLibrary();

        // Auto-remove download entries after extraction — keeps the list clean like Steam
        if (success) {
            setTimeout(() => {
                const games = get(libraryGames);
                const game = games.find(g => g.id === gameId);
                if (game?.download_ids) {
                    for (const downloadId of game.download_ids) {
                        invoke('remove_download', { id: downloadId }).catch(() => {});
                    }
                }
            }, 5000); // 5-second grace period so the user sees "Done"
        }
    });

    // Listen for extraction errors
    await listen<{ gameId: string; error: string }>('extraction-error', (event) => {
        const { gameId, error } = event.payload;

        // Remove from progress tracking
        extractionProgress.update(map => {
            const newMap = new Map(map);
            newMap.delete(gameId);
            return newMap;
        });
    });
}

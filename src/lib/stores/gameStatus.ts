/**
 * Global game status store.
 *
 * Maps (sourceSlug, sourceGameId) → GameGlobalStatus by deriving from the
 * library and downloads stores. No backend call needed — everything is
 * computed reactively from already-loaded state.
 *
 * The key is "${sourceSlug}:${sourceGameId}" where sourceGameId = game_url
 * (set by DownloadButtonsSection when it calls addGameToLibrary).
 */

import { derived } from 'svelte/store';
import { libraryGames } from './library';
import { downloads } from './downloads';
import type { LibraryGame } from '$lib/types';
import type { DownloadDisplay } from './downloads';

export type GamePhase = 'none' | 'downloading' | 'extracting' | 'ready' | 'corrupted';

export interface GameGlobalStatus {
    phase: GamePhase;
    libraryGame?: LibraryGame;
    /** Aggregate download progress 0–100, only set during 'downloading' phase */
    downloadProgress?: number;
    /** Aggregate speed in bytes/s, only set during 'downloading' phase */
    downloadSpeed?: number;
}

/** Derive status map reactively from library + downloads stores. */
export const gameStatusMap = derived(
    [libraryGames, downloads],
    ([$games, $downloads]: [LibraryGame[], DownloadDisplay[]]) => {
        const map = new Map<string, GameGlobalStatus>();

        for (const game of $games) {
            const key = `${game.source_slug}:${game.source_game_id}`;

            let downloadProgress: number | undefined;
            let downloadSpeed: number | undefined;

            if (game.status === 'downloading' && game.download_ids.length > 0) {
                const gameDls = game.download_ids
                    .map(id => $downloads.find(d => d.id === id))
                    .filter((d): d is DownloadDisplay => d !== undefined);

                if (gameDls.length > 0) {
                    const totalBytes = gameDls.reduce((s, d) => s + d.totalBytes, 0);
                    const doneBytes = gameDls.reduce((s, d) => s + d.downloadedBytes, 0);
                    downloadProgress = totalBytes > 0 ? (doneBytes / totalBytes) * 100 : 0;
                    downloadSpeed = gameDls.reduce((s, d) => s + d.speed, 0);
                }
            }

            map.set(key, {
                phase: game.status as GamePhase,
                libraryGame: game,
                downloadProgress,
                downloadSpeed,
            });
        }

        return map;
    }
);

/** Lookup helper — returns { phase: 'none' } when the game is unknown. */
export function statusForGame(
    map: Map<string, GameGlobalStatus>,
    sourceSlug: string,
    sourceGameId: string
): GameGlobalStatus {
    return map.get(`${sourceSlug}:${sourceGameId}`) ?? { phase: 'none' };
}

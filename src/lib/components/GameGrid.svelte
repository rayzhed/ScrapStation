<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import { ChevronLeft, ChevronRight } from 'lucide-svelte';
    import GameCard from './GameCard.svelte';
    import EmptyState from './EmptyState.svelte';
    import { currentPage, hasNextPage, hasPrevPage, goToNextPage, goToPrevPage, loadingSource, isSearchResult } from '$lib/stores/games';
    import { sources, currentSource as currentSourceStore } from '$lib/stores/sources';
    import { gameStatusMap, statusForGame } from '$lib/stores/gameStatus';
    import type { GameCard as GameCardType } from '$lib/types';

    export let games: GameCardType[];
    export let loading: boolean;
    export let error: string;
    export let sourceName: string;
    export let onRetry: (() => void) | undefined = undefined;

    const dispatch = createEventDispatcher();

    $: sourceColor = $sources.find(s => s.id === $currentSourceStore)?.color || '#ffffff';

    function selectGame(index: number) {
        dispatch('select', index);
    }

    const SKELETON_COUNT = 20;
    const skeletons = Array.from({ length: SKELETON_COUNT }, (_, i) => i);
</script>

<div class="browser-root">
    {#if error}
        <EmptyState type="error" {error} {sourceName} {onRetry} />

    {:else if !loading && games.length === 0}
        <EmptyState type="empty" {sourceName} sourceColor={sourceColor} />

    {:else}
        <div class="game-grid">
            {#if loading}
                {#each skeletons as i}
                    <div class="skeleton-card" style="animation-delay: {Math.min(i * 0.028, 0.36)}s;">
                        <div class="shimmer absolute inset-0 rounded-[12px]"></div>
                        <div class="skeleton-footer">
                            <div class="shimmer skeleton-line" style="width: 68%;"></div>
                            <div class="shimmer skeleton-line" style="width: 42%; margin-top: 6px; height: 7px;"></div>
                        </div>
                    </div>
                {/each}

            {:else}
                {#each games as game, index (game.title + game.author + game.game_url)}
                    {@const status = statusForGame($gameStatusMap, $loadingSource, game.game_url)}
                    <div
                        role="button"
                        tabindex="0"
                        on:click={() => selectGame(index)}
                        on:keydown={(e) => e.key === 'Enter' && selectGame(index)}
                    >
                        <GameCard {game} {status} {index} {sourceColor} sourceId={$currentSourceStore} />
                    </div>
                {/each}
            {/if}
        </div>

        {#if !loading && !$isSearchResult}
            <div class="pagination">
                <button
                    on:click={() => goToPrevPage($currentSourceStore)}
                    disabled={!$hasPrevPage}
                    class="btn-secondary page-btn disabled:opacity-25 disabled:cursor-not-allowed"
                >
                    <ChevronLeft size={13} />
                    <span>Previous</span>
                </button>

                <div class="page-label">
                    <span class="page-number">Page {$currentPage}</span>
                    <span class="page-count">{games.length} games</span>
                </div>

                <button
                    on:click={() => goToNextPage($currentSourceStore)}
                    disabled={!$hasNextPage}
                    class="btn-secondary page-btn disabled:opacity-25 disabled:cursor-not-allowed"
                >
                    <span>Next</span>
                    <ChevronRight size={13} />
                </button>
            </div>
        {/if}
    {/if}
</div>

<style>
    .browser-root {
        max-width: 1920px;
        margin: 0 auto;
        padding: 20px 24px 56px;
    }

    /* ── Grid ───────────────────────────────────────────────────────────────── */
    .game-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        gap: 14px;
    }

    /* ── Skeleton cards ─────────────────────────────────────────────────────── */
    .skeleton-card {
        position: relative;
        overflow: hidden;
        aspect-ratio: 3 / 4;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.06);
        background: #1a1a1c;
        opacity: 0;
        animation: sk-appear 0.25s ease forwards;
    }

    @keyframes sk-appear {
        to { opacity: 1; }
    }

    .skeleton-footer {
        position: absolute;
        inset-inline: 0;
        bottom: 0;
        padding: 40px 12px 14px;
        background: linear-gradient(to top, rgba(0, 0, 0, 0.82) 0%, transparent 100%);
    }

    .skeleton-line {
        height: 9px;
        border-radius: 5px;
        display: block;
    }

    /* ── Pagination ─────────────────────────────────────────────────────────── */
    .pagination {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 12px;
        margin-top: 40px;
    }

    .page-btn {
        min-width: 96px;
        justify-content: center;
    }

    .page-label {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 2px;
        min-width: 80px;
    }

    .page-number {
        font-size: 12px;
        font-weight: 600;
        color: var(--label-secondary);
    }

    .page-count {
        font-size: 10px;
        color: var(--label-quaternary);
    }
</style>

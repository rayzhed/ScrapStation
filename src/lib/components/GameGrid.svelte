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
</script>

<div class="mx-auto px-6 py-5" style="max-width: 1920px;">
    {#if loading}
        <EmptyState type="loading" />
    {:else if error}
        <EmptyState
            type="error"
            {error}
            {sourceName}
            {onRetry}
        />
    {:else if games.length === 0}
        <EmptyState type="empty" {sourceName} sourceColor={sourceColor} />
    {:else}
        <div class="grid gap-3.5" style="grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));">
            {#each games as game, index (game.title + game.author + game.game_url)}
                {@const status = statusForGame($gameStatusMap, $loadingSource, game.game_url)}
                <div
                    role="button"
                    tabindex="0"
                    on:click={() => selectGame(index)}
                    on:keydown={(e) => e.key === 'Enter' && selectGame(index)}
                >
                    <GameCard {game} {status} {index} {sourceColor} />
                </div>
            {/each}
        </div>

        <!-- Pagination -->
        {#if !$isSearchResult}
            <div class="flex items-center justify-center gap-4 mt-10">
                <button
                    on:click={() => goToPrevPage($currentSourceStore)}
                    disabled={!$hasPrevPage}
                    class="btn-secondary disabled:opacity-25 disabled:cursor-not-allowed"
                >
                    <ChevronLeft size={14} />
                    <span>Previous</span>
                </button>

                <div class="flex flex-col items-center gap-0.5">
                    <span class="text-[12px] font-semibold" style="color: var(--label-secondary);">
                        Page {$currentPage}
                    </span>
                    <span class="text-[11px]" style="color: var(--label-quaternary);">
                        {games.length} game{games.length === 1 ? '' : 's'}
                    </span>
                </div>

                <button
                    on:click={() => goToNextPage($currentSourceStore)}
                    disabled={!$hasNextPage}
                    class="btn-secondary disabled:opacity-25 disabled:cursor-not-allowed"
                >
                    <span>Next</span>
                    <ChevronRight size={14} />
                </button>
            </div>
        {/if}
    {/if}
</div>

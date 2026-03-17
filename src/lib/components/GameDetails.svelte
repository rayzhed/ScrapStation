<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import { slide } from 'svelte/transition';
    import { ChevronDown, Package, HardDrive, Calendar, Users } from 'lucide-svelte';
    import Tag from './Tag.svelte';
    import type { GameCard } from '$lib/types';

    export let game: GameCard;

    const dispatch = createEventDispatcher();

    function close() {
        dispatch('close');
    }
</script>

<div
    class="fixed inset-x-0 bottom-0 z-50 h-[300px] bg-bg-secondary border-t border-white/8"
    transition:slide={{ duration: 300 }}
>
    <!-- Close -->
    <button
        on:click={close}
        class="absolute top-4 right-5 p-1.5 hover:bg-white/8 rounded-subtle z-10 transition-colors text-white/40 hover:text-white/70"
    >
        <ChevronDown size={18} />
    </button>

    <!-- Content -->
    <div class="relative h-full max-w-[1920px] mx-auto px-10 py-8 flex flex-col gap-4">
        <div class="space-y-2.5">
            <h2 class="text-xl font-bold text-white">{game.title}</h2>

            {#if game.tags?.length > 0}
                <div class="flex flex-wrap gap-1.5">
                    {#each game.tags as tag}
                        <Tag {tag} size="sm" />
                    {/each}
                </div>
            {/if}
        </div>

        <p class="text-xs text-white/50 leading-relaxed line-clamp-2">
            {game.description}
        </p>

        <div class="flex gap-5 text-[11px] text-white/40">
            {#if game.genre}
                <div class="flex items-center gap-1.5">
                    <Package size={12} />
                    <span>{game.genre}</span>
                </div>
            {/if}
            {#if game.size}
                <div class="flex items-center gap-1.5">
                    <HardDrive size={12} />
                    <span>{game.size}</span>
                </div>
            {/if}
            {#if game.release_date}
                <div class="flex items-center gap-1.5">
                    <Calendar size={12} />
                    <span>{game.release_date}</span>
                </div>
            {/if}
            {#if game.game_modes}
                <div class="flex items-center gap-1.5">
                    <Users size={12} />
                    <span>{game.game_modes}</span>
                </div>
            {/if}
        </div>
    </div>
</div>

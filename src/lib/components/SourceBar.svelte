<script lang="ts">
    import { searchDebounceMs, searchMinChars } from '$lib/stores/sources';
    import { Search, X } from 'lucide-svelte';

    export let searchQuery = '';
    export let onSearch: () => void;
    export let onClear: () => void;

    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    function clearDebounce() {
        if (debounceTimer !== null) {
            clearTimeout(debounceTimer);
            debounceTimer = null;
        }
    }

    $: meetsMinChars = searchQuery.trim().length >= $searchMinChars;

    function handleInput() {
        if (!searchQuery.trim()) {
            clearDebounce();
            onClear();
            return;
        }
        if (!meetsMinChars) {
            clearDebounce();
            return;
        }
        clearDebounce();
        debounceTimer = setTimeout(() => {
            debounceTimer = null;
            onSearch();
        }, $searchDebounceMs);
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter') {
            clearDebounce();
            if (meetsMinChars) onSearch();
        } else if (e.key === 'Escape') {
            clearDebounce();
            onClear();
        }
    }

    function handleClear() {
        clearDebounce();
        onClear();
    }
</script>

<div class="relative w-[240px]">
    <Search size={11} class="absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none"
            style="color: var(--label-tertiary);" />
    <input
        type="text"
        placeholder="Search games..."
        bind:value={searchQuery}
        on:input={handleInput}
        on:keydown={handleKeydown}
        class="sc-input w-full pl-7 pr-7"
        style="padding-top: 5px; padding-bottom: 5px; font-size: 12px; border-radius: 8px;"
    />
    {#if searchQuery}
        <button
            on:click={handleClear}
            class="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded-[3px] transition-colors"
            style="color: var(--label-tertiary);"
            on:mouseenter={e => (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.08)'}
            on:mouseleave={e => (e.currentTarget as HTMLElement).style.background = 'transparent'}
        >
            <X size={11} />
        </button>
    {/if}
    {#if searchQuery && !meetsMinChars}
        <p class="absolute left-0 top-full mt-1" style="font-size: 11px; color: var(--label-tertiary);">
            {$searchMinChars - searchQuery.trim().length} more char{$searchMinChars - searchQuery.trim().length === 1 ? '' : 's'} needed
        </p>
    {/if}
</div>

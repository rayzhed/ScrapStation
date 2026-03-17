<script lang="ts">
    import { X } from 'lucide-svelte';
    import { fade } from 'svelte/transition';

    export let error: string;
    export let onClose: () => void;
</script>

{#if error}
    <div
        transition:fade={{ duration: 150 }}
        class="w-full border-l-2 border-accent-danger bg-accent-danger/8"
    >
        <div class="max-w-[1920px] mx-auto px-8 py-3 flex items-center justify-between">
            <div class="flex items-center gap-2.5">
                <div class="w-1.5 h-1.5 rounded-full bg-accent-danger"></div>
                <span class="text-xs font-medium text-white/80">{error}</span>
            </div>
            <button
                on:click={onClose}
                class="p-1 hover:bg-white/8 rounded-subtle transition-colors"
            >
                <X size={13} class="text-white/40" />
            </button>
        </div>
    </div>
{/if}

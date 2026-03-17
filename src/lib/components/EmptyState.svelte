<script lang="ts">
    import { AlertTriangle, RefreshCw, Gamepad2 } from 'lucide-svelte';

    export let type: 'error' | 'empty' | 'loading' = 'empty';
    export let error = '';
    export let sourceName = '';
    export let sourceColor = '';
    export let onRetry: (() => void) | undefined = undefined;
</script>

<div class="flex flex-col items-center justify-center min-h-[500px] gap-5 px-8">
    {#if type === 'error'}
        <div class="w-12 h-12 rounded-[10px] flex items-center justify-center"
             style="border: 1px solid rgba(255,69,58,0.3); background: rgba(255,69,58,0.08);">
            <AlertTriangle size={20} strokeWidth={1.75} style="color: #ff453a;" />
        </div>

        <div class="text-center max-w-md">
            <h2 class="text-[13px] font-semibold mb-2" style="color: var(--label-primary);">Source unavailable</h2>
            <p class="text-[12px] mb-3" style="color: var(--label-tertiary);">Failed to load {sourceName}</p>
            <p class="text-[11px] font-mono px-3 py-2 rounded-[6px]"
               style="color: #ff453a; background: rgba(255,69,58,0.08); border: 1px solid rgba(255,69,58,0.15);">
                {error}
            </p>
        </div>

        {#if onRetry}
            <button on:click={onRetry} class="btn-secondary" style="font-size: 12px;">
                <RefreshCw size={13} strokeWidth={1.75} />
                Retry
            </button>
        {/if}

        <p style="font-size: 11px; color: var(--label-tertiary); margin-top: 4px;">
            Try selecting a different source or check your connection
        </p>

    {:else if type === 'empty'}
        <div class="w-12 h-12 rounded-[10px] flex items-center justify-center"
             style="
                border: 1px solid {sourceColor ? sourceColor + '40' : 'var(--border-subtle)'};
                background: {sourceColor ? sourceColor + '12' : 'rgba(255,255,255,0.04)'};
             ">
            <Gamepad2 size={20} strokeWidth={1.5} style="color: {sourceColor || 'var(--label-quaternary)'}; opacity: 0.7;" />
        </div>

        <div class="text-center">
            <h2 class="text-[13px] font-semibold mb-1.5" style="color: var(--label-tertiary);">No games found</h2>
            <p style="font-size: 12px; color: var(--label-tertiary);">{sourceName ? `Nothing to show for ${sourceName}` : 'Select a source to start browsing'}</p>
        </div>

    {:else}
        <div class="w-5 h-5 rounded-full animate-spin"
             style="border: 1.5px solid rgba(255,255,255,0.1); border-top-color: rgba(255,255,255,0.55);"></div>
        <p style="font-size: 12px; color: var(--label-tertiary);">Loading games…</p>
    {/if}
</div>

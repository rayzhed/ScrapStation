<script lang="ts">
    import { onMount } from 'svelte';
    import { inView, animate } from 'motion';
    import { Gamepad2, CheckCircle2, Download, Loader2, AlertTriangle } from 'lucide-svelte';
    import { getOptimizedImage } from '$lib/utils/imageProcessor';
    import type { GameCard } from '$lib/types';
    import type { GameGlobalStatus } from '$lib/stores/gameStatus';
    import { navigateTo } from '$lib/stores/navigation';

    export let game: GameCard;
    export let selected = false;
    export let status: GameGlobalStatus = { phase: 'none' };
    export let index = 0;
    export let sourceColor = '#ffffff';

    let cardEl: HTMLDivElement;
    let imageSrc = '';
    let imageLoading = true;
    let imageError = false;

    $: loadImage(game.cover_url);

    async function loadImage(url: string) {
        imageLoading = true;
        imageError = false;
        try {
            imageSrc = await getOptimizedImage(url);
        } catch {
            imageError = true;
        } finally {
            imageLoading = false;
        }
    }

    onMount(() => {
        const stop = inView(
            cardEl,
            () => {
                animate(
                    cardEl,
                    { opacity: [0, 1], y: [16, 0], scale: [0.97, 1] },
                    { duration: 0.45, delay: Math.min(index * 0.035, 0.3), easing: [0.22, 1, 0.36, 1] }
                );
                stop();
            },
            { amount: 0.05 }
        );
        return stop;
    });

    function handleMouseEnter() {
        animate(cardEl, { y: -4, scale: 1.02 }, { duration: 0.28, easing: [0.34, 1.56, 0.64, 1] });
        cardEl.style.borderColor = sourceColor + '55';
        cardEl.style.boxShadow = '0 2px 8px rgba(0,0,0,0.6), 0 12px 32px rgba(0,0,0,0.6), 0 24px 64px rgba(0,0,0,0.4)';
    }

    function handleMouseLeave() {
        animate(cardEl, { y: 0, scale: 1 }, { duration: 0.22, easing: 'ease-out' });
        cardEl.style.borderColor = selected ? 'rgba(255,255,255,0.30)' : 'rgba(255,255,255,0.08)';
        cardEl.style.boxShadow = '0 1px 3px rgba(0,0,0,0.5), 0 4px 16px rgba(0,0,0,0.3)';
    }

    function handleNavigate(e: MouseEvent) {
        e.stopPropagation();
        if (status.phase === 'ready' || status.phase === 'corrupted' || status.phase === 'extracting') {
            navigateTo('library');
        } else if (status.phase === 'downloading') {
            navigateTo('downloads');
        }
    }

    $: hasStatus = status.phase !== 'none';
    $: downloadPct = status.phase === 'downloading' && status.downloadProgress != null
        ? Math.round(status.downloadProgress)
        : null;
</script>

<div
    bind:this={cardEl}
    class="group relative overflow-hidden cursor-pointer select-none"
    style="
        aspect-ratio: 3/4;
        border-radius: 12px;
        border: 1px solid {selected ? 'rgba(255,255,255,0.30)' : 'rgba(255,255,255,0.08)'};
        box-shadow: 0 1px 3px rgba(0,0,0,0.5), 0 4px 16px rgba(0,0,0,0.3);
        opacity: 0;
        will-change: transform;
        background: #1c1c1e;
        transition: border-color 0.18s ease;
        {selected ? 'box-shadow: 0 0 0 2px rgba(255,255,255,0.14), 0 8px 24px rgba(0,0,0,0.6);' : ''}
    "
    role="button"
    tabindex="0"
    on:mouseenter={handleMouseEnter}
    on:mouseleave={handleMouseLeave}
>
    <!-- Cover image / skeleton -->
    {#if imageLoading}
        <div class="absolute inset-0 bg-[#1c1c1e]">
            <div class="shimmer absolute inset-0"></div>
        </div>
    {:else if imageError || !imageSrc}
        <div class="absolute inset-0 flex flex-col items-center justify-center bg-[#1c1c1e]">
            <Gamepad2 size={32} strokeWidth={1.25} style="color: rgba(235,235,245,0.12);" />
        </div>
    {:else}
        <img
            src={imageSrc}
            alt={game.title}
            class="absolute inset-0 w-full h-full object-cover transition-transform duration-700 ease-out group-hover:scale-[1.06]"
        />
    {/if}

    <!-- Source color ambient wash — fades naturally from top (no hard line) -->
    <div
        class="absolute top-0 inset-x-0 z-10 pointer-events-none"
        style="height: 56px; background: linear-gradient(to bottom, {sourceColor}1e 0%, transparent 100%);"
    ></div>

    <!-- Bottom gradient — title area -->
    <div
        class="absolute inset-x-0 bottom-0 z-10 px-3 pb-3 pt-12"
        style="background: linear-gradient(to top,
            rgba(0,0,0,0.93) 0%,
            rgba(0,0,0,0.72) 38%,
            rgba(0,0,0,0.18) 72%,
            transparent 100%);"
    >
        <!-- Status badge -->
        {#if hasStatus}
            <div class="mb-1.5">
                {#if status.phase === 'ready'}
                    <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-[5px] text-[11px] font-semibold"
                          style="background: rgba(50,215,75,0.18); color: #32d74b; border: 1px solid rgba(50,215,75,0.28);">
                        <CheckCircle2 size={10} strokeWidth={2.5} />
                        Installed
                    </span>
                {:else if status.phase === 'downloading'}
                    <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-[5px] text-[11px] font-semibold"
                          style="background: rgba(10,132,255,0.18); color: #0a84ff; border: 1px solid rgba(10,132,255,0.28);">
                        <Download size={10} strokeWidth={2.5} />
                        {downloadPct != null ? `${downloadPct}%` : 'Downloading'}
                    </span>
                {:else if status.phase === 'extracting'}
                    <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-[5px] text-[11px] font-semibold"
                          style="background: rgba(191,90,242,0.18); color: #bf5af2; border: 1px solid rgba(191,90,242,0.28);">
                        <Loader2 size={10} strokeWidth={2.5} class="animate-spin" />
                        Installing
                    </span>
                {:else if status.phase === 'corrupted'}
                    <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-[5px] text-[11px] font-semibold"
                          style="background: rgba(255,69,58,0.18); color: #ff453a; border: 1px solid rgba(255,69,58,0.28);">
                        <AlertTriangle size={10} strokeWidth={2.5} />
                        Error
                    </span>
                {/if}
            </div>
        {/if}

        <h3
            class="text-[12px] font-semibold leading-snug line-clamp-2"
            style="color: #fff; letter-spacing: -0.01em; text-shadow: 0 1px 6px rgba(0,0,0,0.9);"
            title={game.title}
        >
            {game.title}
        </h3>

        <!-- Download progress bar -->
        {#if status.phase === 'downloading' && downloadPct != null}
            <div class="mt-2 h-[2px] rounded-full overflow-hidden" style="background: rgba(255,255,255,0.12);">
                <div class="h-full rounded-full transition-all duration-500"
                     style="width: {downloadPct}%; background: #0a84ff;"></div>
            </div>
        {/if}
    </div>

    <!-- Hover overlay for status navigation -->
    {#if hasStatus}
        <div
            class="absolute inset-0 z-20 flex items-center justify-center
                   opacity-0 group-hover:opacity-100 transition-opacity duration-200"
            style="background: rgba(0,0,0,0.52);"
        >
            <button
                on:click={handleNavigate}
                class="px-4 py-2 rounded-[8px] text-[11px] font-semibold transition-all hover:scale-105 active:scale-95"
                style="
                    letter-spacing: -0.01em;
                    {status.phase === 'ready'
                        ? 'background: rgba(50,215,75,0.18); color: #32d74b; border: 1px solid rgba(50,215,75,0.35);'
                        : status.phase === 'corrupted'
                        ? 'background: rgba(255,159,10,0.18); color: #ff9f0a; border: 1px solid rgba(255,159,10,0.35);'
                        : 'background: rgba(10,132,255,0.18); color: #0a84ff; border: 1px solid rgba(10,132,255,0.35);'}
                "
            >
                {status.phase === 'ready' ? 'View in Library'
                 : status.phase === 'downloading' ? 'View Downloads'
                 : 'View in Library'}
            </button>
        </div>
    {/if}
</div>

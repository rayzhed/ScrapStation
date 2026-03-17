<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { onMount } from 'svelte';
    import { animate, inView } from 'motion';
    import SectionRenderer from './detail-sections/SectionRenderer.svelte';
    import { X, CheckCircle2, Download, Loader2, AlertTriangle, Library, ArrowRight } from 'lucide-svelte';
    import { gameStatusMap, statusForGame } from '$lib/stores/gameStatus';
    import { navigateTo } from '$lib/stores/navigation';
    import { formatBytes } from '$lib/stores/downloads';
    import { getOptimizedImage } from '$lib/utils/imageProcessor';
    import NoticeModal from './NoticeModal.svelte';

    export let game: any;
    export let sourceId: string;
    export let onClose: () => void;

    let sections: any[] = [];
    let loading = true;
    let error = '';
    let pendingDetailNotice: any | null = null;
    let archivePassword: string | undefined = undefined;
    let ambientSrc = '';
    let rootEl: HTMLDivElement;

    $: status = statusForGame($gameStatusMap, sourceId, game?.game_url ?? '');

    // Load ambient cover art
    $: if (game?.cover_url) {
        getOptimizedImage(game.cover_url)
            .then(src => { ambientSrc = src; })
            .catch(() => {});
    }

    onMount(() => {
        // Entrance animation
        animate(
            rootEl,
            { opacity: [0, 1], y: [14, 0] },
            { duration: 0.35, easing: [0.22, 1, 0.36, 1] }
        );
    });

    // Svelte action: stagger sections in via inView
    function sectionIn(node: HTMLElement, index: number) {
        node.style.opacity = '0';
        const stop = inView(node, () => {
            animate(
                node,
                { opacity: [0, 1], y: [18, 0] },
                { duration: 0.42, delay: Math.min(index * 0.06, 0.28), easing: [0.22, 1, 0.36, 1] }
            );
            stop();
        }, { amount: 0.08 });
        return { destroy: () => stop() };
    }

    function extractArchivePassword(secs: any[]): string | undefined {
        let found: { label: string; value: string }[] = [];
        for (const section of secs) {
            if (section.type === 'metadata_grid' && section.data?.items) {
                for (const item of section.data.items) {
                    const label = (item.label || '').toLowerCase();
                    const value = (item.value || '').trim();
                    if (value && (label.includes('password') || label.includes('пароль') || label.includes('global'))) {
                        let v = value;
                        if (v.includes(':')) {
                            const after = v.split(':').slice(1).join(':').trim();
                            if (after) v = after;
                        }
                        found.push({ label: item.label, value: v });
                    }
                }
            }
        }
        const archivePass = found.find(p => p.label.toLowerCase().includes('archive'));
        if (archivePass) return archivePass.value;
        if (found.length > 0) return found[0].value;
        return undefined;
    }

    async function loadDetails() {
        try {
            loading = true;
            error = '';
            const [secs, meta] = await Promise.all([
                invoke<any[]>('get_game_detail_sections', { gameUrl: game.game_url, sourceId }),
                invoke<any>('get_source_metadata', { sourceId }).catch(() => ({ notices: [] })),
            ]);
            sections = secs;
            archivePassword = extractArchivePassword(sections);
            checkDetailViewNotices(meta.notices || []);
        } catch (e) {
            error = String(e);
        } finally {
            loading = false;
        }
    }

    function checkDetailViewNotices(notices: any[]) {
        for (const n of notices) {
            if (n.trigger !== 'detail_view') continue;
            if (n.once && localStorage.getItem(`notice_seen_${sourceId}_${n.id}`)) continue;
            pendingDetailNotice = n;
            break;
        }
    }

    $: if (game && sourceId) { loadDetails(); }

    function goToSection() {
        onClose();
        navigateTo(status.phase === 'downloading' ? 'downloads' : 'library');
    }

    function handleClose() {
        animate(rootEl, { opacity: [1, 0], y: [0, 10] }, { duration: 0.22, easing: 'ease-in' })
            .finished.then(onClose);
    }
</script>

{#if pendingDetailNotice}
    <NoticeModal
        notice={pendingDetailNotice}
        sourceId={sourceId}
        onConfirm={() => { pendingDetailNotice = null; }}
    />
{/if}

<!-- ── iOS 26 Liquid Glass overlay ───────────────────────────────── -->
<div
    bind:this={rootEl}
    class="detail-root"
    style="
        --bg-surface: rgba(14, 14, 18, 0.72);
        --bg-raised: rgba(22, 22, 28, 0.68);
        --border: rgba(255,255,255,0.13);
        --border-subtle: rgba(255,255,255,0.07);
        opacity: 0;
    "
>
    <!-- ── Ambient background ─────────────────────────────────── -->
    <div class="ambient-layer" aria-hidden="true">
        {#if ambientSrc}
            <div class="ambient-art" style="background-image: url({ambientSrc});"></div>
        {/if}
        <div class="ambient-veil"></div>
        <!-- Specular rim — glass catching light -->
        <div class="ambient-rim"></div>
    </div>

    <!-- ── Scrollable content ─────────────────────────────────── -->
    <div class="content-scroll">
        <div class="content-inner">

            <!-- Close button — glass pill -->
            <button class="close-btn" on:click={handleClose} aria-label="Close">
                <X size={14} strokeWidth={2.5} />
            </button>

            <!-- Status banner -->
            {#if status.phase !== 'none' && !loading}
                {@const bc = status.phase === 'ready' ? '#32d74b'
                    : status.phase === 'corrupted' ? '#ff9f0a'
                    : status.phase === 'downloading' ? '#0a84ff'
                    : 'rgba(245,245,247,0.50)'}
                <div class="status-banner sc-card" style="border-color: {bc}28;">
                    <div class="shrink-0">
                        {#if status.phase === 'ready'}
                            <CheckCircle2 size={15} strokeWidth={2} style="color: #32d74b;" />
                        {:else if status.phase === 'downloading'}
                            <Download size={15} strokeWidth={2} style="color: #0a84ff;" />
                        {:else if status.phase === 'extracting'}
                            <Loader2 size={15} strokeWidth={2} class="animate-spin" style="color: var(--label-tertiary);" />
                        {:else if status.phase === 'corrupted'}
                            <AlertTriangle size={15} strokeWidth={2} style="color: #ff9f0a;" />
                        {/if}
                    </div>
                    <div class="flex-1 min-w-0">
                        {#if status.phase === 'ready'}
                            <p class="text-[12px] font-medium" style="color: #32d74b;">Game installed in your library</p>
                            <p class="text-[11px] mt-0.5" style="color: var(--label-tertiary);">
                                {status.libraryGame?.install_size ? formatBytes(status.libraryGame.install_size) : 'Ready to play'}
                            </p>
                        {:else if status.phase === 'downloading'}
                            <p class="text-[12px] font-medium" style="color: #0a84ff;">Downloading…</p>
                            {#if status.downloadProgress != null}
                                <div class="flex items-center gap-2 mt-1">
                                    <div class="flex-1 h-[2px] rounded-full overflow-hidden" style="background: rgba(255,255,255,0.1);">
                                        <div class="h-full rounded-full transition-all duration-300"
                                             style="width: {status.downloadProgress}%; background: #0a84ff;"></div>
                                    </div>
                                    <span class="text-[11px] tabular-nums" style="color: var(--label-tertiary);">
                                        {Math.round(status.downloadProgress)}%
                                    </span>
                                </div>
                            {/if}
                        {:else if status.phase === 'extracting'}
                            <p class="text-[12px] font-medium" style="color: var(--label-secondary);">Installing game files…</p>
                        {:else if status.phase === 'corrupted'}
                            <p class="text-[12px] font-medium" style="color: #ff9f0a;">Installation error</p>
                        {/if}
                    </div>
                    <button
                        on:click={goToSection}
                        class="banner-action"
                        style="border-color: {bc}30; color: {bc};"
                        on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = `${bc}12`; }}
                        on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
                    >
                        {#if status.phase === 'ready'}
                            <Library size={11} strokeWidth={2} />Open Library
                        {:else if status.phase === 'downloading'}
                            <Download size={11} strokeWidth={2} />View Downloads
                        {:else}
                            <Library size={11} strokeWidth={2} />Open Library
                        {/if}
                        <ArrowRight size={10} strokeWidth={2.5} />
                    </button>
                </div>
            {/if}

            <!-- Loading -->
            {#if loading}
                <div class="flex flex-col items-center justify-center min-h-[400px] gap-4">
                    <div class="w-7 h-7 rounded-full animate-spin"
                         style="border: 1.5px solid rgba(255,255,255,0.08); border-top-color: rgba(255,255,255,0.45);"></div>
                    <p class="text-[12px]" style="color: var(--label-tertiary);">Loading game details…</p>
                </div>

            <!-- Error -->
            {:else if error}
                <div class="text-center py-12">
                    <div class="inline-flex items-center justify-center w-12 h-12 rounded-[10px] mb-4"
                         style="border: 1px solid rgba(255,69,58,0.28); background: rgba(255,69,58,0.08);">
                        <X size={18} style="color: #ff453a;" />
                    </div>
                    <h3 class="text-[13px] font-semibold mb-2" style="color: var(--label-primary);">Failed to load details</h3>
                    <p class="text-[12px] mb-5" style="color: var(--label-tertiary);">{error}</p>
                    <button on:click={handleClose} class="btn-secondary">Close</button>
                </div>

            <!-- Empty -->
            {:else if sections.length === 0}
                <div class="text-center py-12">
                    <p class="text-[12px] mb-5" style="color: var(--label-tertiary);">No details available for this game.</p>
                    <button on:click={handleClose} class="btn-secondary">Close</button>
                </div>

            <!-- Sections — staggered inView entrance -->
            {:else}
                <div>
                    {#each sections as section, i (section.order)}
                        <div use:sectionIn={i}>
                            <SectionRenderer
                                {section}
                                {sourceId}
                                gameTitle={game.title || 'Unknown Game'}
                                gameUrl={game.game_url}
                                coverUrl={game.cover_url || ''}
                                {archivePassword}
                            />
                        </div>
                    {/each}
                </div>
            {/if}

        </div>
    </div>
</div>

<style>
    /* ── Root overlay ─────────────────────────────────────────── */
    .detail-root {
        position: fixed;
        inset: 0;
        top: 32px;
        z-index: 50;
        overflow: hidden;
        /* Solid base so the game grid doesn't bleed through */
        background: rgb(6, 6, 9);
    }

    /* ── Ambient background ───────────────────────────────────── */
    .ambient-layer {
        position: absolute;
        inset: 0;
        overflow: hidden;
    }
    /* Cover art extends well beyond bounds to prevent blur edge artifacts */
    .ambient-art {
        position: absolute;
        top: -160px;
        left: -160px;
        right: -160px;
        bottom: -160px;
        background-size: cover;
        background-position: center;
        filter: blur(80px) saturate(200%);
        opacity: 0.55;
    }
    /* Semi-transparent dark veil — light enough to let the art's colors show */
    .ambient-veil {
        position: absolute;
        inset: 0;
        background: rgba(6, 6, 9, 0.58);
    }
    /* Specular rim — 1px gradient highlight at the top edge */
    .ambient-rim {
        position: absolute;
        inset-x: 0;
        top: 0;
        height: 1px;
        background: linear-gradient(
            to right,
            transparent 0%,
            rgba(255,255,255,0.10) 20%,
            rgba(255,255,255,0.22) 50%,
            rgba(255,255,255,0.10) 80%,
            transparent 100%
        );
    }

    /* ── Scrollable content ───────────────────────────────────── */
    .content-scroll {
        position: relative;
        z-index: 10;
        height: 100%;
        overflow-y: auto;
        scrollbar-width: thin;
        scrollbar-color: rgba(255,255,255,0.10) transparent;
    }
    .content-inner {
        padding: 40px clamp(20px, 5vw, 64px) 72px;
        max-width: 860px;
        margin: 0 auto;
    }

    /* ── Close button — frosted glass pill ────────────────────── */
    .close-btn {
        position: fixed;
        top: 44px;
        right: 20px;
        z-index: 60;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 30px;
        height: 30px;
        border-radius: 50%;
        border: 1px solid rgba(255,255,255,0.16);
        background: rgba(255,255,255,0.08);
        color: rgba(245,245,247,0.65);
        backdrop-filter: blur(20px) saturate(140%);
        -webkit-backdrop-filter: blur(20px) saturate(140%);
        cursor: pointer;
        transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
    }
    .close-btn:hover {
        background: rgba(255,255,255,0.14);
        border-color: rgba(255,255,255,0.26);
        color: rgba(245,245,247,0.92);
    }
    .close-btn:active { transform: scale(0.92); }

    /* ── Status banner ────────────────────────────────────────── */
    .status-banner {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 10px 14px;
        margin-bottom: 24px;
        backdrop-filter: blur(20px) saturate(140%);
        -webkit-backdrop-filter: blur(20px) saturate(140%);
    }

    /* ── Banner action button ─────────────────────────────────── */
    .banner-action {
        flex-shrink: 0;
        display: inline-flex;
        align-items: center;
        gap: 5px;
        padding: 5px 10px;
        border-radius: 6px;
        border: 1px solid;
        font-size: 11px;
        font-weight: 500;
        background: transparent;
        cursor: pointer;
        letter-spacing: -0.01em;
        transition: background 0.15s ease;
    }

    /* ── All sc-card children become frosted glass panels ─────── */
    .detail-root :global(.sc-card) {
        backdrop-filter: blur(20px) saturate(140%);
        -webkit-backdrop-filter: blur(20px) saturate(140%);
    }
</style>

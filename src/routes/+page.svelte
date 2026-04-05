<script lang="ts">
    import { onMount } from 'svelte';
    import { loadAvailableSources, initSourceWatcher, currentSource, sources } from '$lib/stores/sources';
    import { games, loading, error, loadGames, searchGames, loadingSource } from '$lib/stores/games';
    import { initDownloadEvents } from '$lib/stores/downloads';
    import { initLibraryEvents } from '$lib/stores/library';
    import { loadStorageConfig } from '$lib/stores/appSettings';
    import { checkForUpdates } from '$lib/stores/updater';
    import { currentMode } from '$lib/stores/navigation';
    import { RotateCw, FolderOpen, Minus, Square, X as XIcon } from 'lucide-svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { getVersion } from '@tauri-apps/api/app';
    import { animate } from 'motion';

    import Header from '$lib/components/Header.svelte';
    import SourceBar from '$lib/components/SourceBar.svelte';
    import GameGrid from '$lib/components/GameGrid.svelte';
    import GameDetailsPage from '$lib/components/GameDetailsPage.svelte';
    import DownloadsPage from '$lib/components/DownloadsPage.svelte';
    import LibraryPage from '$lib/components/LibraryPage.svelte';
    import UpdatesPage from '$lib/components/UpdatesPage.svelte';
    import AboutPage from '$lib/components/AboutPage.svelte';
    import SettingsManager from '$lib/components/SettingsManager.svelte';

    const appWindow = getCurrentWindow();
    let isMaximized = false;
    let isDevBuild = false;

    async function refreshMaximized() { isMaximized = await appWindow.isMaximized(); }
    async function minimize()         { await appWindow.minimize(); }
    async function toggleMaximize()   { await appWindow.toggleMaximize(); await refreshMaximized(); }
    async function close()            { await appWindow.close(); }

    let searchQuery = '';
    let selectedGameIndex = -1;
    let isRefreshing = false;

    async function refreshGames() {
        if (!$currentSource || isRefreshing) return;
        isRefreshing = true;
        try { await loadGames($currentSource, 1); } finally { isRefreshing = false; }
    }

    async function openSourcesFolder() {
        await invoke('open_sources_folder').catch(() => {});
    }

    onMount(async () => {
        const v = await getVersion().catch(() => '');
        isDevBuild = v.includes('-dev');
        await loadAvailableSources();
        await initDownloadEvents();
        await initLibraryEvents();
        await initSourceWatcher();
        loadStorageConfig().catch(() => {});
        // Silent update check — result is reflected in updateState store
        checkForUpdates().catch(() => {});
    });

    function handleSearch() {
        if (searchQuery.trim() && $currentSource) searchGames($currentSource, searchQuery);
    }
    function clearSearch() {
        searchQuery = '';
        if ($currentSource) loadGames($currentSource, 1);
    }
    function handleRetry() {
        if ($currentSource) loadGames($currentSource, 1);
    }

    $: currentSourceName  = $sources.find(s => s.id === $currentSource)?.name || '';
    $: currentSourceColor = $sources.find(s => s.id === $currentSource)?.color || '#f5f5f7';
    $: loadingSourceName  = $sources.find(s => s.id === $loadingSource)?.name || $loadingSource;
    $: pageTitle = $currentMode === 'updates'   ? 'Updates'
                 : $currentMode === 'browse'    ? 'Browse'
                 : $currentMode === 'library'   ? 'Library'
                 : $currentMode === 'settings'  ? 'Settings'
                 : $currentMode === 'about'     ? 'About'
                 : 'Downloads';

    // Svelte action: Motion-powered page entrance
    function pageIn(node: HTMLElement) {
        node.style.opacity = '0';
        animate(
            node,
            { opacity: [0, 1], y: [10, 0] },
            { duration: 0.30, easing: [0.22, 1, 0.36, 1] }
        );
    }
</script>

<!-- ── Titlebar ────────────────────────────────────────────────────────────── -->
<div
    class="fixed top-0 left-0 right-0 z-[200] flex items-stretch"
    class:dev-titlebar={isDevBuild}
    style="
        height: 32px;
        background: var(--bg-sidebar);
        border-bottom: 1px solid var(--border-subtle);
    "
    data-tauri-drag-region
>
    <!-- Drag fill -->
    <div class="flex-1" data-tauri-drag-region></div>

    <!-- Dev mode label -->
    {#if isDevBuild}
        <div
            class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 pointer-events-none"
            style="font-size: 10px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; color: rgba(245,245,247,0.3);"
        >
            Development Build
        </div>
    {/if}

    <!-- Window controls — Windows style -->
    <div class="flex items-stretch" data-tauri-drag-region="false">
        {#each [
            { action: minimize,        icon: Minus,   label: 'Minimize' },
            { action: toggleMaximize,  icon: Square,  label: isMaximized ? 'Restore' : 'Maximize' },
            { action: close,           icon: XIcon,   label: 'Close', danger: true },
        ] as btn}
            <button
                on:click={btn.action}
                title={btn.label}
                class="flex items-center justify-center transition-colors"
                style="
                    width: 44px;
                    color: rgba(245,245,247,0.45);
                    background: transparent;
                    border: none;
                "
                on:mouseenter={e => {
                    const el = e.currentTarget as HTMLElement;
                    el.style.background = btn.danger ? '#c42b1c' : 'rgba(255,255,255,0.07)';
                    el.style.color = btn.danger ? '#fff' : 'rgba(245,245,247,0.85)';
                }}
                on:mouseleave={e => {
                    const el = e.currentTarget as HTMLElement;
                    el.style.background = 'transparent';
                    el.style.color = 'rgba(245,245,247,0.45)';
                }}
            >
                <svelte:component this={btn.icon} size={11} strokeWidth={1.75} />
            </button>
        {/each}
    </div>
</div>

<!-- ── App shell ───────────────────────────────────────────────────────────── -->
<main class="flex overflow-hidden" style="height: 100vh; padding-top: 32px; background: var(--bg-page);">

    <!-- Sidebar -->
    <Header />

    <!-- Content column -->
    <div class="flex flex-col flex-1 min-w-0 overflow-hidden">

        <!-- Topbar -->
        <div
            class="flex items-center shrink-0 gap-2 px-5"
            style="
                height: 44px;
                background: var(--bg-page);
                border-bottom: 1px solid var(--border-subtle);
            "
        >
            <!-- Breadcrumb -->
            <span style="font-size: 13px; font-weight: 600; color: var(--label-primary); letter-spacing: -0.015em;">
                {pageTitle}
            </span>

            {#if $currentMode === 'browse' && $games.length > 0}
                <span class="num" style="
                    font-size: 11px; font-weight: 600;
                    padding: 1px 6px; border-radius: 5px;
                    background: rgba(255,255,255,0.07);
                    color: var(--label-tertiary);
                    border: 1px solid var(--border-subtle);
                ">
                    {$games.length}
                </span>
            {/if}

            {#if $currentMode === 'browse' && currentSourceName}
                <span style="color: var(--label-quaternary); font-size: 13px; line-height: 1;">/</span>
                <span style="font-size: 13px; font-weight: 500; color: {currentSourceColor}; letter-spacing: -0.01em;">
                    {currentSourceName}
                </span>
            {/if}

            <div class="flex-1"></div>

            {#if $currentMode === 'browse'}
                <div class="flex items-center gap-0.5 mr-1">
                    <button
                        on:click={refreshGames}
                        disabled={isRefreshing}
                        class="btn-icon"
                        style="width: 30px; height: 30px;"
                        title="Refresh games"
                    >
                        <RotateCw size={13} strokeWidth={1.75} class={isRefreshing ? 'animate-spin' : ''} />
                    </button>
                    <button
                        on:click={openSourcesFolder}
                        class="btn-icon"
                        style="width: 30px; height: 30px;"
                        title="Open sources folder"
                    >
                        <FolderOpen size={13} strokeWidth={1.75} />
                    </button>
                </div>

                <SourceBar
                    bind:searchQuery
                    onSearch={handleSearch}
                    onClear={clearSearch}
                />
            {/if}
        </div>

        <!-- Page content -->
        <div class="flex-1 overflow-hidden">
            {#key $currentMode}
                <div use:pageIn class="h-full overflow-y-auto">
                    {#if $currentMode === 'updates'}
                        <UpdatesPage />
                    {:else if $currentMode === 'browse'}
                        <GameGrid
                            games={$games}
                            loading={$loading}
                            error={$error}
                            sourceName={loadingSourceName}
                            onRetry={handleRetry}
                            on:select={e => selectedGameIndex = e.detail}
                        />
                    {:else if $currentMode === 'library'}
                        <LibraryPage />
                    {:else if $currentMode === 'downloads'}
                        <DownloadsPage />
                    {:else if $currentMode === 'about'}
                        <AboutPage />
                    {:else if $currentMode === 'settings'}
                        <SettingsManager />
                    {/if}
                </div>
            {/key}
        </div>
    </div>

    <!-- Game detail overlay -->
    {#if selectedGameIndex >= 0 && $games[selectedGameIndex] && $currentSource}
        <GameDetailsPage
            game={$games[selectedGameIndex]}
            sourceId={$currentSource}
            onClose={() => selectedGameIndex = -1}
        />
    {/if}
</main>

<style>
    .dev-titlebar::before {
        content: '';
        position: absolute;
        inset: 0;
        pointer-events: none;
        background: repeating-linear-gradient(
            -45deg,
            transparent 0px,
            transparent 16px,
            rgba(255, 255, 255, 0.055) 16px,
            rgba(255, 255, 255, 0.055) 32px
        );
        background-size: 45.3px 45.3px;
        animation: stripe-scroll 1.4s linear infinite;
    }

    @keyframes stripe-scroll {
        from { background-position: 0 0; }
        to   { background-position: 45.3px 0; }
    }
</style>

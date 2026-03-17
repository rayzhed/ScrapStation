<script lang="ts">
    import { onMount } from 'svelte';
    import { fly, fade } from 'svelte/transition';
    import { animate } from 'motion';
    import {
        downloads,
        downloadGroups,
        downloadStats,
        removeDownload,
        clearFinished,
        pauseDownload,
        resumeDownload,
        cancelDownload,
        openFileLocation as openFileLoc,
        openDownloadFolder,
        formatBytes,
        formatSpeed,
        formatDuration,
        formatETA,
        initDownloadEvents,
        getArchivePaths,
        type DownloadDisplay,
    } from '$lib/stores/downloads';
    import {
        extractToLibrary,
        extractionProgress,
        libraryGames,
    } from '$lib/stores/library';
    import type { DownloadGroup } from '$lib/types';
    import {
        Download as DownloadIcon,
        CheckCircle2,
        XCircle,
        Trash2,
        FolderOpen,
        Clock,
        Zap,
        RotateCcw,
        X,
        Pause,
        Play,
        ChevronDown,
        ChevronUp,
        StopCircle,
        Package,
        Archive,
        Loader2,
        FileArchive,
        AlertCircle,
        Activity,
        HardDrive,
    } from 'lucide-svelte';

    let activeTab: 'all' | 'active' | 'completed' | 'failed' = 'all';
    let expandedGroup: string | null = null;
    let extractingGroups = new Set<string>();

    // Svelte action: staggered row entrance
    function rowIn(node: HTMLElement, index: number) {
        node.style.opacity = '0';
        animate(
            node,
            { opacity: [0, 1], y: [12, 0] },
            { duration: 0.35, delay: Math.min(index * 0.05, 0.30), easing: [0.22, 1, 0.36, 1] }
        );
    }

    onMount(() => {
        initDownloadEvents();
    });

    function getFilteredGroups(tab: string, groups: DownloadGroup[]): DownloadGroup[] {
        switch (tab) {
            case 'active':
                return groups.filter(g => g.status === 'downloading' || g.status === 'queued' || g.status === 'pending' || g.status === 'paused' || g.status === 'extracting');
            case 'completed':
                return groups.filter(g => g.status === 'completed');
            case 'failed':
                return groups.filter(g => g.status === 'failed');
            default:
                return groups;
        }
    }

    async function handleOpenFileLocation(filePath: string) {
        try { await openFileLoc(filePath); } catch (e) { }
    }

    async function handlePauseAll(group: DownloadGroup) {
        for (const dl of group.downloads) {
            if (dl.status === 'downloading') {
                try { await pauseDownload(dl.id); } catch (e) { }
            }
        }
    }

    async function handleResumeAll(group: DownloadGroup) {
        for (const dl of group.downloads) {
            if (dl.status === 'paused') {
                try { await resumeDownload(dl.id); } catch (e) { }
            }
        }
    }

    async function handleCancelAll(group: DownloadGroup) {
        for (const dl of group.downloads) {
            if (dl.status === 'downloading' || dl.status === 'pending' || dl.status === 'paused') {
                try { await cancelDownload(dl.id); } catch (e) { }
            }
        }
    }

    async function handleRemoveGroup(group: DownloadGroup) {
        for (const dl of group.downloads) {
            try { await removeDownload(dl.id); } catch (e) { }
        }
    }

    function findLibraryGameForGroup(group: DownloadGroup): { id: string; password?: string } | null {
        const downloadIds = group.downloads.map(d => d.id);
        for (const game of $libraryGames) {
            if (game.download_ids && game.download_ids.some(id => downloadIds.includes(id))) {
                return { id: game.id, password: game.archive_password };
            }
        }
        return null;
    }

    async function handleExtract(group: DownloadGroup) {
        if (extractingGroups.has(group.gameId)) return;
        const archivePaths = getArchivePaths(group);
        if (archivePaths.length === 0) return;
        const libraryGame = findLibraryGameForGroup(group);
        if (!libraryGame) return;
        extractingGroups.add(group.gameId);
        extractingGroups = extractingGroups;
        try {
            await extractToLibrary(libraryGame.id, archivePaths, libraryGame.password);
        } catch (e) {
        } finally {
            extractingGroups.delete(group.gameId);
            extractingGroups = extractingGroups;
        }
    }

    async function handleClearFinished() {
        try { await clearFinished(); } catch (e) { }
    }

    function toggleExpand(id: string) {
        expandedGroup = expandedGroup === id ? null : id;
    }

    function getStatusColor(status: string): string {
        switch (status) {
            case 'downloading': return '#0a84ff';
            case 'completed':   return '#32d74b';
            case 'failed':      return '#ff453a';
            case 'queued':      return 'rgba(245,245,247,0.5)';
            case 'pending':     return '#ff9f0a';
            case 'paused':      return '#ff9f0a';
            case 'extracting':  return '#bf5af2';
            default:            return 'rgba(245,245,247,0.3)';
        }
    }

    function getStatusIcon(status: string) {
        switch (status) {
            case 'downloading': return DownloadIcon;
            case 'completed':   return CheckCircle2;
            case 'failed':      return XCircle;
            case 'queued':      return Clock;
            case 'pending':     return Clock;
            case 'paused':      return Pause;
            case 'extracting':  return Archive;
            default:            return DownloadIcon;
        }
    }

    function getDownloadStatusIcon(status: string) {
        switch (status) {
            case 'downloading': return DownloadIcon;
            case 'completed':   return CheckCircle2;
            case 'failed':      return XCircle;
            case 'cancelled':   return StopCircle;
            case 'pending':     return Clock;
            case 'paused':      return Pause;
            default:            return DownloadIcon;
        }
    }

    function getProgressGradient(status: string): string {
        switch (status) {
            case 'downloading': return '#0a84ff';
            case 'extracting':  return '#bf5af2';
            case 'paused':      return '#ff9f0a';
            default:            return '#32d74b';
        }
    }

    $: groups = $downloadGroups;
    $: filteredGroups = getFilteredGroups(activeTab, groups);
    $: stats = $downloadStats;
    $: progress = $extractionProgress;
</script>

<div class="p-5">
    <div style="max-width: 900px; margin: 0 auto;">

        <!-- Active download stats bar -->
        {#if stats.activeCount > 0}
            <div
                class="mb-4 sc-card flex items-center gap-4 px-4 py-3"
                style="border-color: rgba(10,132,255,0.2); box-shadow: 0 0 0 1px rgba(10,132,255,0.06), 0 2px 8px rgba(0,0,0,0.4);"
                in:fly={{ y: -8, duration: 250 }}
            >
                <div class="flex items-center gap-2">
                    <div class="w-2 h-2 rounded-full animate-pulse" style="background: #0a84ff;"></div>
                    <span class="text-xs font-semibold" style="color: #0a84ff; letter-spacing: -0.01em;">
                        {stats.activeCount} Active
                    </span>
                </div>
                <div class="w-px h-3.5" style="background: rgba(255,255,255,0.08);"></div>
                <div class="flex items-center gap-1.5">
                    <Zap size={12} style="color: var(--label-tertiary);" />
                    <span class="text-sm font-bold num" style="color: #ffffff; letter-spacing: -0.02em;">{formatSpeed(stats.totalSpeed)}</span>
                </div>
                <div class="ml-auto flex items-center gap-2">
                    {#if stats.completedCount > 0 || stats.failedCount > 0}
                        <button on:click={handleClearFinished} class="btn-secondary" style="padding: 4px 10px; font-size: 11px;">
                            <Trash2 size={11} />
                            Clear finished
                        </button>
                    {/if}
                    <button on:click={openDownloadFolder} class="btn-secondary" style="padding: 4px 10px; font-size: 11px;">
                        <FolderOpen size={11} />
                        Open folder
                    </button>
                </div>
            </div>
        {:else}
            <!-- Quiet header when nothing active -->
            <div class="mb-4 flex items-center justify-between">
                <p class="text-[12px]" style="color: var(--label-tertiary); letter-spacing: -0.01em;">
                    {stats.completedCount} completed · {stats.failedCount} failed
                </p>
                <div class="flex gap-2">
                    {#if stats.completedCount > 0 || stats.failedCount > 0}
                        <button on:click={handleClearFinished} class="btn-secondary" style="padding: 4px 10px; font-size: 11px;">
                            <Trash2 size={11} />
                            Clear finished
                        </button>
                    {/if}
                    <button on:click={openDownloadFolder} class="btn-secondary" style="padding: 4px 10px; font-size: 11px;">
                        <FolderOpen size={11} />
                        Open folder
                    </button>
                </div>
            </div>
        {/if}

        <!-- Tabs -->
        <div class="flex gap-1 mb-4" style="border-bottom: 1px solid rgba(255,255,255,0.06);">

            {#each [
                { id: 'all' as const,       label: 'All',       count: groups.length },
                { id: 'active' as const,    label: 'Active',    count: groups.filter(g => g.status !== 'completed' && g.status !== 'failed').length },
                { id: 'completed' as const, label: 'Completed', count: groups.filter(g => g.status === 'completed').length },
                { id: 'failed' as const,    label: 'Failed',    count: groups.filter(g => g.status === 'failed').length }
            ] as tab}
                <button
                    on:click={() => activeTab = tab.id}
                    class="sc-tab {activeTab === tab.id ? 'active' : ''}"
                    style="padding-left: 2px; padding-right: 2px; margin-right: 14px;"
                >
                    {tab.label}
                    {#if tab.count > 0}
                        <span class="num" style="font-size: 11px; padding: 1px 5px; border-radius: 4px; font-weight: 600;
                                     background: {activeTab === tab.id ? 'rgba(255,255,255,0.1)' : 'rgba(255,255,255,0.05)'};
                                     color: {activeTab === tab.id ? 'var(--label-secondary)' : 'var(--label-tertiary)'};">
                            {tab.count}
                        </span>
                    {/if}
                </button>
            {/each}
        </div>

        <!-- Downloads List -->
        {#if filteredGroups.length === 0}
            <div class="flex flex-col items-center justify-center py-20 text-center" in:fade={{ duration: 200 }}>
                <div class="w-14 h-14 rounded-[14px] flex items-center justify-center mb-4"
                     style="background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.07);">
                    <DownloadIcon size={20} style="color: rgba(235,235,245,0.15);" />
                </div>
                <p class="text-[12px] font-semibold mb-1.5" style="color: var(--label-tertiary); letter-spacing: -0.01em;">No downloads</p>
                <p class="text-[11px] max-w-xs" style="color: var(--label-tertiary); line-height: 1.5;">
                    {activeTab === 'active' ? 'No active downloads. Browse games to start.'
                    : activeTab === 'completed' ? 'No completed downloads yet.'
                    : activeTab === 'failed' ? 'No failed downloads.'
                    : 'Your downloads will appear here.'}
                </p>
            </div>
        {:else}
            <div class="space-y-2">
                {#each filteredGroups as group, rowIndex (group.gameId)}
                    {@const StatusIcon = getStatusIcon(group.status)}
                    {@const statusColor = getStatusColor(group.status)}
                    {@const isExpanded = expandedGroup === group.gameId}
                    {@const isExtracting = extractingGroups.has(group.gameId) || group.status === 'extracting'}
                    {@const extractProgress = progress.get(group.libraryGameId || '')}
                    {@const totalSpeed = group.downloads.reduce((sum, d) => sum + (d.status === 'downloading' ? d.speed : 0), 0)}
                    {@const progressGrad = getProgressGradient(group.status)}

                    <div
                        use:rowIn={rowIndex}
                        class="sc-card overflow-hidden transition-all duration-200"
                        style="border-color: {
                            group.status === 'downloading' ? 'rgba(10,132,255,0.2)'
                            : group.status === 'extracting' ? 'rgba(191,90,242,0.2)'
                            : group.status === 'completed' ? 'rgba(50,215,75,0.12)'
                            : group.status === 'failed' ? 'rgba(255,69,58,0.15)'
                            : 'var(--border)'
                        }; box-shadow: var(--shadow-card);"
                    >
                        <!-- Colored top accent -->
                        <div style="height: 2px; background: {statusColor}; opacity: {group.status === 'downloading' || group.status === 'extracting' ? '0.6' : '0.25'};"></div>

                        <!-- Main Row -->
                        <div class="px-4 py-3.5 flex items-center gap-3">
                            <!-- Status icon -->
                            <div
                                class="w-9 h-9 rounded-subtle flex items-center justify-center shrink-0"
                                style="border: 1px solid {statusColor}30; background: {statusColor}0a;"
                            >
                                {#if isExtracting}
                                    <Loader2 size={16} style="color: {statusColor};" class="animate-spin" />
                                {:else}
                                    <svelte:component this={StatusIcon} size={16} style="color: {statusColor};" />
                                {/if}
                            </div>

                            <!-- Info + Progress -->
                            <div class="flex-1 min-w-0">
                                <div class="flex items-center gap-2 mb-0.5">
                                    <h3 class="text-[12px] font-semibold truncate" style="color: var(--label-primary);">{group.gameTitle}</h3>
                                    {#if group.isMultiPart}
                                        <span class="px-1.5 py-px rounded-[3px] shrink-0 flex items-center gap-1"
                                              style="font-size: 11px; background: rgba(255,255,255,0.07); color: var(--label-tertiary); border: 1px solid var(--border-subtle);">
                                            <FileArchive size={10} />
                                            Multi-part
                                        </span>
                                    {/if}
                                </div>

                                <p class="text-[11px] truncate mb-2" style="color: var(--label-tertiary);">
                                    {group.downloads.length === 1 ? group.downloads[0].fileName : `${group.downloads.length} files`}
                                </p>

                                <!-- Progress bar -->
                                {#if group.status !== 'completed' && group.status !== 'failed' && group.status !== 'queued'}
                                    <div class="flex items-center gap-2">
                                        <div class="flex-1 h-[3px] bg-white/8 rounded-full overflow-hidden relative">
                                            {#if isExtracting && extractProgress}
                                                <div
                                                    class="h-full rounded-full transition-all duration-700 ease-out"
                                                    style="width: {(extractProgress.bytes_done / extractProgress.bytes_total * 100)}%; background: {progressGrad};"
                                                ></div>
                                            {:else if group.progress >= 0 && group.totalSize > 0}
                                                <div
                                                    class="h-full rounded-full transition-all duration-500 ease-out"
                                                    style="width: {group.progress}%; background: {progressGrad};"
                                                ></div>
                                            {:else if group.status === 'downloading' || group.status === 'pending' || group.progress < 0}
                                                <div
                                                    class="absolute h-full w-1/3 rounded-full animate-indeterminate"
                                                    style="background: {progressGrad};"
                                                ></div>
                                            {:else if group.status === 'paused'}
                                                <div
                                                    class="h-full rounded-full"
                                                    style="width: {group.progress}%; background: {progressGrad}; opacity: 0.5;"
                                                ></div>
                                            {/if}
                                        </div>
                                        <span class="text-[11px] shrink-0 w-10 text-right tabular-nums font-mono" style="color: var(--label-tertiary);">
                                            {#if isExtracting && extractProgress}
                                                {(extractProgress.bytes_done / extractProgress.bytes_total * 100).toFixed(0)}%
                                            {:else if group.progress >= 0 && group.totalSize > 0}
                                                {group.progress.toFixed(0)}%
                                            {:else if group.status === 'paused'}
                                                —
                                            {:else}
                                                —
                                            {/if}
                                        </span>
                                    </div>
                                {/if}
                            </div>

                            <!-- Right stats -->
                            <div class="flex items-center gap-4 shrink-0">
                                {#if group.status === 'downloading'}
                                    <div class="text-right">
                                        <div class="text-[12px] font-bold tabular-nums" style="color: {statusColor};">{formatSpeed(totalSpeed)}</div>
                                        <div class="text-[11px] tabular-nums mt-0.5" style="color: var(--label-tertiary);">
                                            {formatBytes(group.downloadedSize)}{group.totalSize > 0 ? ` / ${formatBytes(group.totalSize)}` : ''}
                                        </div>
                                    </div>
                                {:else if isExtracting}
                                    <div class="text-right">
                                        <div class="text-[11px] font-bold" style="color: {statusColor};">Extracting</div>
                                        {#if extractProgress}
                                            <div class="text-[11px] tabular-nums mt-0.5" style="color: var(--label-tertiary);">
                                                {extractProgress.files_done}/{extractProgress.files_total} files
                                            </div>
                                        {/if}
                                    </div>
                                {:else if group.status === 'queued'}
                                    <span class="text-[11px]" style="color: var(--label-tertiary);">Queued</span>
                                {:else if group.status === 'paused'}
                                    <div class="text-right">
                                        <div class="text-[11px] font-bold" style="color: #ff9f0a;">Paused</div>
                                        <div class="text-[11px] tabular-nums mt-0.5" style="color: var(--label-tertiary);">{formatBytes(group.downloadedSize)} / {formatBytes(group.totalSize)}</div>
                                    </div>
                                {:else if group.status === 'completed'}
                                    <div class="text-right">
                                        <div class="text-[11px] font-bold tabular-nums" style="color: #32d74b;">{formatBytes(group.totalSize)}</div>
                                        <div class="text-[11px] mt-0.5 flex items-center gap-1 justify-end" style="color: rgba(50,215,75,0.6);">
                                            <CheckCircle2 size={10} />
                                            Done
                                        </div>
                                    </div>
                                {:else if group.status === 'failed'}
                                    <div class="text-[11px] flex items-center gap-1.5" style="color: #ff453a;">
                                        <AlertCircle size={12} />
                                        Failed
                                    </div>
                                {/if}

                                <!-- Actions -->
                                <div class="flex items-center gap-1">
                                    {#if group.status === 'completed' && group.canExtract && !isExtracting}
                                        <button
                                            on:click={() => handleExtract(group)}
                                            class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-[6px] text-[11px] transition-colors"
                                            style="border: 1px solid rgba(191,90,242,0.3); color: #bf5af2;"
                                            on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(191,90,242,0.1)'; }}
                                            on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
                                            title="Extract to library"
                                        >
                                            <Archive size={12} />
                                            Extract
                                        </button>
                                    {/if}

                                    {#if group.status === 'completed'}
                                        <button
                                            on:click={() => {
                                                const dl = group.downloads.find(d => d.filePath);
                                                if (dl?.filePath) handleOpenFileLocation(dl.filePath);
                                            }}
                                            class="p-1.5 rounded-[6px] transition-colors"
                                            style="border: 1px solid var(--border); color: var(--label-tertiary);"
                                            on:mouseenter={e => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--label-primary)'; el.style.borderColor = 'var(--border-strong)'; }}
                                            on:mouseleave={e => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--label-tertiary)'; el.style.borderColor = 'var(--border)'; }}
                                            title="Open file location"
                                        >
                                            <FolderOpen size={13} />
                                        </button>
                                    {/if}

                                    {#if group.status === 'downloading'}
                                        <button
                                            on:click={() => handlePauseAll(group)}
                                            class="p-1.5 rounded-[6px] transition-colors"
                                            style="border: 1px solid var(--border); color: var(--label-tertiary);"
                                            on:mouseenter={e => { const el = e.currentTarget as HTMLElement; el.style.color = '#ff9f0a'; el.style.borderColor = 'rgba(255,159,10,0.3)'; }}
                                            on:mouseleave={e => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--label-tertiary)'; el.style.borderColor = 'var(--border)'; }}
                                            title="Pause"
                                        >
                                            <Pause size={13} />
                                        </button>
                                    {/if}

                                    {#if group.status === 'paused'}
                                        <button
                                            on:click={() => handleResumeAll(group)}
                                            class="p-1.5 rounded-[6px] transition-colors"
                                            style="border: 1px solid rgba(10,132,255,0.3); color: #0a84ff;"
                                            on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(10,132,255,0.1)'; }}
                                            on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
                                            title="Resume"
                                        >
                                            <Play size={13} />
                                        </button>
                                    {/if}

                                    {#if group.status === 'failed'}
                                        <button
                                            on:click={() => handleResumeAll(group)}
                                            class="p-1.5 rounded-[6px] transition-colors"
                                            style="border: 1px solid var(--border); color: var(--label-tertiary);"
                                            on:mouseenter={e => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--label-primary)'; el.style.borderColor = 'var(--border-strong)'; }}
                                            on:mouseleave={e => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--label-tertiary)'; el.style.borderColor = 'var(--border)'; }}
                                            title="Retry"
                                        >
                                            <RotateCcw size={13} />
                                        </button>
                                    {/if}

                                    {#if group.status === 'downloading' || group.status === 'pending'}
                                        <button
                                            on:click={() => handleCancelAll(group)}
                                            class="p-1.5 rounded-[6px] transition-colors"
                                            style="border: 1px solid var(--border); color: var(--label-tertiary);"
                                            on:mouseenter={e => { const el = e.currentTarget as HTMLElement; el.style.color = '#ff453a'; el.style.borderColor = 'rgba(255,69,58,0.3)'; }}
                                            on:mouseleave={e => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--label-tertiary)'; el.style.borderColor = 'var(--border)'; }}
                                            title="Cancel"
                                        >
                                            <StopCircle size={13} />
                                        </button>
                                    {/if}

                                    {#if group.status === 'completed' || group.status === 'failed' || group.status === 'paused'}
                                        <button
                                            on:click={() => handleRemoveGroup(group)}
                                            class="p-1.5 rounded-[6px] transition-colors"
                                            style="border: 1px solid var(--border); color: var(--label-tertiary);"
                                            on:mouseenter={e => { const el = e.currentTarget as HTMLElement; el.style.color = '#ff453a'; el.style.borderColor = 'rgba(255,69,58,0.3)'; }}
                                            on:mouseleave={e => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--label-tertiary)'; el.style.borderColor = 'var(--border)'; }}
                                            title="Remove"
                                        >
                                            <X size={13} />
                                        </button>
                                    {/if}

                                    {#if group.downloads.length > 1}
                                        <button
                                            on:click={() => toggleExpand(group.gameId)}
                                            class="p-1.5 rounded-[6px] transition-colors"
                                            style="border: 1px solid var(--border); color: var(--label-tertiary);"
                                            on:mouseenter={e => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--label-primary)'; el.style.borderColor = 'var(--border-strong)'; }}
                                            on:mouseleave={e => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--label-tertiary)'; el.style.borderColor = 'var(--border)'; }}
                                        >
                                            {#if isExpanded}
                                                <ChevronUp size={13} />
                                            {:else}
                                                <ChevronDown size={13} />
                                            {/if}
                                        </button>
                                    {/if}
                                </div>
                            </div>
                        </div>

                        <!-- Expanded File List -->
                        {#if isExpanded && group.downloads.length > 1}
                            <div class="px-4 pb-3 border-t border-white/5" in:fly={{ y: -4, duration: 200 }}>
                                <div class="pt-3 space-y-1.5">
                                    {#each group.downloads as download (download.id)}
                                        {@const dlStatusColor = getStatusColor(download.status)}
                                        {@const DlStatusIcon = getDownloadStatusIcon(download.status)}
                                        <div class="flex items-center gap-2.5 p-2.5 rounded-subtle bg-black/30 border border-white/5">
                                            <svelte:component this={DlStatusIcon} size={11} style="color: {dlStatusColor};" />
                                            <div class="flex-1 min-w-0">
                                                <p class="text-[11px] truncate" style="color: var(--label-secondary);">{download.fileName}</p>
                                                {#if download.status === 'downloading' && download.totalBytes > 0}
                                                    <div class="mt-1 h-[2px] bg-white/8 rounded-full overflow-hidden">
                                                        <div
                                                            class="h-full rounded-full transition-all duration-500"
                                                            style="width: {(download.downloadedBytes / download.totalBytes * 100)}%; background: {dlStatusColor};"
                                                        ></div>
                                                    </div>
                                                {/if}
                                            </div>
                                            <div class="text-[11px] tabular-nums shrink-0" style="color: var(--label-tertiary);">
                                                {#if download.status === 'downloading'}
                                                    {formatSpeed(download.speed)} · {formatBytes(download.downloadedBytes)}/{formatBytes(download.totalBytes)}
                                                {:else if download.status === 'completed'}
                                                    {formatBytes(download.totalBytes)}
                                                {:else if download.status === 'paused'}
                                                    Paused · {formatBytes(download.downloadedBytes)}/{formatBytes(download.totalBytes)}
                                                {:else if download.status === 'failed'}
                                                    {download.error || 'Failed'}
                                                {:else}
                                                    Pending
                                                {/if}
                                            </div>
                                            <span
                                                class="px-1.5 py-0.5 rounded-[3px] font-semibold"
                                                style="font-size: 11px; color: {download.hostColor}; border: 1px solid {download.hostColor}40;"
                                            >
                                                {download.hostLabel}
                                            </span>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {/if}
                    </div>
                {/each}
            </div>
        {/if}
    </div>
</div>


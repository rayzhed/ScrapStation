<script lang="ts">
    import { onMount } from 'svelte';
    import { fly, fade, scale } from 'svelte/transition';
    import { inView, animate } from 'motion';
    import {
        readyGames,
        libraryStats,
        launchGame,
        removeFromLibrary,
        setGameExecutable,
        openGameFolder,
        openLibraryFolder,
        moveGame,
        rescanGameExecutables,
        repairLibrary,
        initLibraryEvents,
        formatPlaytime,
        formatSize,
        getBestCoverSource,
    } from '$lib/stores/library';
    import {
        Library,
        Play,
        Trash2,
        FolderOpen,
        Settings2,
        Clock,
        HardDrive,
        Gamepad2,
        RotateCcw,
        Check,
        AlertTriangle,
        Loader2,
        X,
        Wrench,
    } from 'lucide-svelte';
    import { getKnownLibraryLocations, type KnownLibraryLocation } from '$lib/stores/appSettings';
    import { getOptimizedImage } from '$lib/utils/imageProcessor';
    import type { LibraryGame } from '$lib/types';

    let selectedGame: LibraryGame | null = null;
    let showExeSelector = false;
    let isLaunching = false;
    let deleteTarget: LibraryGame | null = null;
    let isDeleting = false;
    let isRepairing = false;
    let repairResult: number | null = null;
    let folderError: string | null = null;
    let isMoving = false;
    let moveResult: { success: boolean; message: string } | null = null;
    let showMoveDropdown = false;
    let knownLocations: KnownLibraryLocation[] = [];
    let loadingLocations = false;

    function handleWindowClick() {
        if (showMoveDropdown) showMoveDropdown = false;
    }

    onMount(() => { initLibraryEvents(); });

    async function handleLaunch(game: LibraryGame) {
        if (isLaunching) return;
        isLaunching = true;
        try { await launchGame(game.id); }
        catch (e) { }
        finally { setTimeout(() => isLaunching = false, 1000); }
    }

    function handleRemove(game: LibraryGame) { deleteTarget = game; }

    async function confirmDelete() {
        if (!deleteTarget || isDeleting) return;
        isDeleting = true;
        try {
            await removeFromLibrary(deleteTarget.id, true);
            selectedGame = null;
            deleteTarget = null;
        } finally { isDeleting = false; }
    }

    function cancelDelete() { deleteTarget = null; }

    async function handleSetExecutable(game: LibraryGame, exePath: string) {
        await setGameExecutable(game.id, exePath);
        showExeSelector = false;
    }

    async function handleRescan(game: LibraryGame) { await rescanGameExecutables(game.id); }

    async function handleRepair() {
        if (isRepairing) return;
        isRepairing = true;
        repairResult = null;
        try {
            repairResult = await repairLibrary();
        } catch (e) {
            repairResult = 0;
        } finally {
            isRepairing = false;
            setTimeout(() => { repairResult = null; }, 4000);
        }
    }

    async function handleOpenLibraryFolder() {
        try { await openLibraryFolder(); } catch (e) {}
    }

    function gameDrive(installPath: string): string {
        const m = installPath.match(/^([A-Za-z]):[/\\]/);
        return m ? m[1].toUpperCase() + ':' : '';
    }

    function gameLibraryRoot(installPath: string): string {
        // install_path is absolute: .../Library/game-slug/game
        // strip the last two path components to get the Library folder
        const normalized = installPath.replace(/\\/g, '/');
        const parts = normalized.split('/').filter(Boolean);
        // remove trailing "game" and "game-slug"
        return parts.slice(0, -2).join('/').toLowerCase();
    }

    async function toggleMoveDropdown(game: LibraryGame) {
        if (showMoveDropdown) {
            showMoveDropdown = false;
            return;
        }
        loadingLocations = true;
        showMoveDropdown = true;
        try {
            const locs = await getKnownLibraryLocations();
            const gameRoot = gameLibraryRoot(game.install_path);
            knownLocations = locs.filter(l =>
                l.path.replace(/\\/g, '/').toLowerCase() !== gameRoot
            );
        } catch (e) {
            knownLocations = [];
        } finally {
            loadingLocations = false;
        }
    }

    async function handleMoveToLocation(game: LibraryGame, location: KnownLibraryLocation) {
        showMoveDropdown = false;
        isMoving = true;
        moveResult = null;
        try {
            await moveGame(game.id, location.path);
            moveResult = { success: true, message: `Moved to ${location.label}.` };
        } catch (e) {
            moveResult = { success: false, message: String(e) };
        } finally {
            isMoving = false;
            setTimeout(() => { moveResult = null; }, 5000);
        }
    }

    async function handleOpenGameFolder(game: LibraryGame) {
        folderError = null;
        try {
            await openGameFolder(game.id);
        } catch (e) {
            folderError = String(e);
            setTimeout(() => { folderError = null; }, 6000);
        }
    }

    function selectGame(game: LibraryGame) {
        selectedGame = selectedGame?.id === game.id ? null : game;
        showExeSelector = false;
    }

    function getExeTypeLabel(type: string): string {
        switch (type) {
            case 'main': return 'Main';
            case 'launcher': return 'Launcher';
            case 'tool': return 'Tool';
            default: return '';
        }
    }

    function animateCard(node: HTMLElement, { index }: { index: number }) {
        // Set initial opacity here (not in the style attribute) so Svelte
        // reactive style updates (border/shadow on selection) don't reset it.
        node.style.opacity = '0';
        const stop = inView(node, () => {
            animate(node,
                { opacity: [0, 1], y: [12, 0], scale: [0.96, 1] },
                { duration: 0.4, delay: Math.min(index * 0.035, 0.3), easing: [0.22, 1, 0.36, 1] }
            );
            stop();
        }, { amount: 0.05 });
        return { destroy: stop };
    }

    $: stats = $libraryStats;
    $: ready = $readyGames;
</script>

<svelte:window on:click={handleWindowClick} />

<div class="p-5 pb-32">
    <div class="max-w-[1600px] mx-auto">

        <!-- Stats bar -->
        {#if stats.gameCount > 0}
            <div class="mb-5 flex items-center gap-4 px-4 py-2.5"
                 style="background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;">
                <div class="flex items-center gap-1.5">
                    <Library size={12} style="color: var(--label-tertiary);" />
                    <span style="font-size: 12px; font-weight: 500; color: var(--label-secondary); letter-spacing: -0.01em;">
                        {stats.gameCount} {stats.gameCount === 1 ? 'game' : 'games'}
                    </span>
                </div>
                {#if stats.totalSize > 0}
                    <div class="w-px h-3" style="background: var(--border-subtle);"></div>
                    <div class="flex items-center gap-1 num" style="font-size: 11px; color: var(--label-tertiary);">
                        <HardDrive size={11} />
                        {formatSize(stats.totalSize)}
                    </div>
                {/if}
                {#if stats.totalPlaytime > 0}
                    <div class="w-px h-3" style="background: var(--border-subtle);"></div>
                    <div class="flex items-center gap-1 num" style="font-size: 11px; color: var(--label-tertiary);">
                        <Clock size={11} />
                        {formatPlaytime(stats.totalPlaytime)} played
                    </div>
                {/if}
                <!-- Library folder + fix buttons -->
                <div class="ml-auto flex items-center gap-2">
                    {#if repairResult !== null}
                        <span style="font-size: 11px; color: var(--label-tertiary);" in:fade={{ duration: 200 }}>
                            {repairResult === 0 ? 'Nothing to recover' : `${repairResult} game${repairResult === 1 ? '' : 's'} recovered`}
                        </span>
                    {/if}
                    <button
                        on:click={handleOpenLibraryFolder}
                        class="flex items-center gap-1.5 transition-colors"
                        style="font-size: 11px; color: var(--label-tertiary); padding: 4px 8px; border-radius: 6px;
                               border: 1px solid var(--border-subtle); background: transparent;"
                        title="Open Library folder"
                        on:mouseenter={e => (e.currentTarget as HTMLElement).style.color = 'var(--label-secondary)'}
                        on:mouseleave={e => (e.currentTarget as HTMLElement).style.color = 'var(--label-tertiary)'}
                    >
                        <FolderOpen size={10} />
                        Library
                    </button>
                    <button
                        on:click={handleRepair}
                        disabled={isRepairing}
                        class="flex items-center gap-1.5 transition-colors disabled:opacity-40"
                        style="font-size: 11px; color: var(--label-tertiary); padding: 4px 8px; border-radius: 6px;
                               border: 1px solid var(--border-subtle); background: transparent;"
                        title="Scan Library folder for missing games"
                        on:mouseenter={e => (e.currentTarget as HTMLElement).style.color = 'var(--label-secondary)'}
                        on:mouseleave={e => (e.currentTarget as HTMLElement).style.color = 'var(--label-tertiary)'}
                    >
                        {#if isRepairing}
                            <Loader2 size={10} class="animate-spin" />
                        {:else}
                            <Wrench size={10} />
                        {/if}
                        Fix
                    </button>
                </div>
            </div>
        {/if}

        <!-- Folder error banner -->
        {#if folderError}
            <div class="mb-4 px-3 py-2 rounded-subtle flex items-start gap-2 text-[11px]"
                 style="background: rgba(255,59,48,0.08); border: 1px solid rgba(255,59,48,0.2); color: #ff6b6b;"
                 in:fade={{ duration: 150 }} out:fade={{ duration: 150 }}>
                <AlertTriangle size={13} class="flex-shrink-0 mt-0.5" />
                {folderError}
            </div>
        {/if}

        <!-- Empty State -->
        {#if ready.length === 0}
            <div class="flex flex-col items-center justify-center py-32 text-center" in:fade={{ duration: 300 }}>
                <div class="w-[72px] h-[72px] rounded-[20px] flex items-center justify-center mb-5"
                     style="background: rgba(255,255,255,0.04); border: 1px solid var(--border-subtle);">
                    <Library size={28} style="color: var(--label-quaternary);" />
                </div>
                <p style="font-size: 14px; font-weight: 600; color: var(--label-tertiary); letter-spacing: -0.02em; margin-bottom: 6px;">
                    Your library is empty
                </p>
                <p style="font-size: 12px; color: var(--label-tertiary); max-width: 220px; line-height: 1.6;">
                    Games you download and install will appear here.
                </p>
                <!-- Fix / recover button -->
                <div class="mt-6 flex flex-col items-center gap-2">
                    <button
                        on:click={handleRepair}
                        disabled={isRepairing}
                        class="flex items-center gap-2 transition-all disabled:opacity-40"
                        style="padding: 8px 16px; font-size: 12px; font-weight: 600; letter-spacing: -0.01em;
                               border-radius: 8px; border: 1px solid var(--border);
                               background: rgba(255,255,255,0.05); color: var(--label-secondary);"
                        on:mouseenter={e => (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.09)'}
                        on:mouseleave={e => (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.05)'}
                    >
                        {#if isRepairing}
                            <Loader2 size={13} class="animate-spin" />
                            Scanning…
                        {:else}
                            <Wrench size={13} />
                            Fix Library
                        {/if}
                    </button>
                    {#if repairResult !== null}
                        <p style="font-size: 11px; color: var(--label-tertiary);" in:fade={{ duration: 200 }}>
                            {repairResult === 0 ? 'No game folders found to recover.' : `${repairResult} game${repairResult === 1 ? '' : 's'} recovered.`}
                        </p>
                    {:else}
                        <p style="font-size: 11px; color: var(--label-quaternary);">
                            Scan the Library folder for existing game installs
                        </p>
                    {/if}
                </div>
            </div>

        {:else}
            <!-- Game Grid — poster style -->
            <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(148px, 1fr));">
                {#each ready as game, i (game.id)}
                    {@const isSelected = selectedGame?.id === game.id}
                    {@const drive = gameDrive(game.install_path)}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div
                        class="relative group cursor-pointer"
                        style="
                            aspect-ratio: 3/4;
                            border-radius: 12px;
                            overflow: hidden;
                            background: #1c1c1e;
                            border: 1px solid {isSelected ? 'rgba(255,255,255,0.3)' : 'var(--border-subtle)'};
                            box-shadow: {isSelected
                                ? '0 0 0 2px rgba(255,255,255,0.12), 0 12px 32px rgba(0,0,0,0.6)'
                                : 'var(--shadow-card)'};
                            transition: border-color 0.2s, box-shadow 0.2s;
                        "
                        use:animateCard={{ index: i }}
                        role="button"
                        tabindex="0"
                        on:click={() => selectGame(game)}
                    >
                        <!-- Cover image -->
                        {#if getBestCoverSource(game)}
                            {#await getOptimizedImage(getBestCoverSource(game)) then src}
                                <img
                                    src={src}
                                    alt={game.title}
                                    class="absolute inset-0 w-full h-full object-cover transition-transform duration-700 group-hover:scale-[1.05]"
                                />
                            {:catch}
                                <div class="absolute inset-0 flex items-center justify-center">
                                    <Gamepad2 size={28} style="color: var(--label-quaternary);" />
                                </div>
                            {/await}
                        {:else}
                            <div class="absolute inset-0 flex items-center justify-center">
                                <Gamepad2 size={28} style="color: var(--label-quaternary);" />
                            </div>
                        {/if}

                        <!-- Bottom gradient + info -->
                        <div class="absolute inset-x-0 bottom-0 z-10 px-2.5 pb-2.5 pt-10"
                             style="background: linear-gradient(to top, rgba(0,0,0,0.9) 0%, rgba(0,0,0,0.55) 50%, transparent 100%);">
                            <h3 style="font-size: 11px; font-weight: 600; letter-spacing: -0.01em; line-height: 1.3;
                                       color: #fff; text-shadow: 0 1px 4px rgba(0,0,0,0.8);"
                                class="line-clamp-2">
                                {game.title}
                            </h3>
                            <div class="flex items-center gap-1 mt-1 num" style="font-size: 11px; color: var(--label-tertiary);">
                                <HardDrive size={10} />
                                <span>{formatSize(game.install_size)}</span>
                                {#if game.total_playtime > 0}
                                    <span style="color: var(--label-quaternary);">·</span>
                                    <Clock size={10} />
                                    <span>{formatPlaytime(game.total_playtime)}</span>
                                {/if}
                                {#if drive}
                                    <span style="margin-left: auto; font-size: 9px; font-weight: 700;
                                                 letter-spacing: 0.04em; padding: 1px 4px; border-radius: 3px;
                                                 background: rgba(255,255,255,0.1); color: rgba(255,255,255,0.5);">
                                        {drive}
                                    </span>
                                {/if}
                            </div>
                        </div>

                        <!-- Hover overlay -->
                        <div class="absolute inset-0 z-20 flex flex-col items-center justify-center gap-2.5
                                    opacity-0 group-hover:opacity-100 transition-opacity duration-200"
                             style="background: rgba(0,0,0,0.58);">
                            <button
                                on:click|stopPropagation={() => handleLaunch(game)}
                                class="w-12 h-12 rounded-full flex items-center justify-center shadow-lg
                                       transform scale-90 group-hover:scale-100 transition-transform duration-200
                                       hover:scale-105 disabled:opacity-40"
                                style="background: #ffffff;"
                                title="Play"
                                disabled={isLaunching || !game.primary_exe}
                            >
                                {#if isLaunching}
                                    <Loader2 size={16} class="animate-spin" style="color: #000;" />
                                {:else}
                                    <Play size={17} fill="#000" style="color: #000; margin-left: 2px;" />
                                {/if}
                            </button>
                            <button
                                on:click|stopPropagation={() => handleOpenGameFolder(game)}
                                class="flex items-center justify-center transition-colors"
                                style="width: 28px; height: 28px; border-radius: 8px;
                                       background: rgba(255,255,255,0.1); border: 1px solid rgba(255,255,255,0.14);
                                       color: var(--label-secondary);"
                                title="Open folder"
                            >
                                <FolderOpen size={12} />
                            </button>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>
</div>

<!-- Selected Game Panel — slides up from bottom -->
{#if selectedGame}
    {@const game = selectedGame}
    <div
        class="fixed bottom-0 right-0 z-50 px-4 pb-4"
        style="left: 196px;"
        in:fly={{ y: 60, duration: 320, easing: t => 1 - Math.pow(1 - t, 3) }}
        out:fly={{ y: 60, duration: 220 }}
    >
        <!-- Panel -->
        <div style="
            background: var(--bg-surface);
            border: 1px solid var(--border);
            border-radius: 14px;
            box-shadow: var(--shadow-panel);
        ">
            <!-- Top accent -->
            <div style="height: 2px; background: rgba(255,255,255,0.1); border-radius: 14px 14px 0 0;"></div>

            {#if moveResult}
                <div class="flex items-center gap-2 px-5 py-2.5"
                     style="font-size: 11px; border-bottom: 1px solid var(--border-subtle);
                            background: {moveResult.success ? 'rgba(50,215,75,0.06)' : 'rgba(255,69,58,0.06)'};
                            color: {moveResult.success ? '#32d74b' : '#ff453a'};
                            border-radius: 14px 14px 0 0;"
                     in:fly={{ y: -4, duration: 180 }}>
                    {#if moveResult.success}
                        <Check size={12} />
                    {:else}
                        <AlertTriangle size={12} />
                    {/if}
                    {moveResult.message}
                </div>
            {/if}

            <div class="flex items-center gap-4 px-5 py-4">
                <!-- Cover thumbnail -->
                <div style="width: 44px; height: 58px; border-radius: 8px; overflow: hidden; flex-shrink: 0;
                             background: var(--bg-raised); border: 1px solid var(--border-subtle);">
                    {#if getBestCoverSource(game)}
                        {#await getOptimizedImage(getBestCoverSource(game)) then src}
                            <img src={src} alt={game.title} class="w-full h-full object-cover" />
                        {:catch}
                            <div class="w-full h-full flex items-center justify-center">
                                <Gamepad2 size={16} style="color: var(--label-quaternary);" />
                            </div>
                        {/await}
                    {:else}
                        <div class="w-full h-full flex items-center justify-center">
                            <Gamepad2 size={16} style="color: var(--label-quaternary);" />
                        </div>
                    {/if}
                </div>

                <!-- Game info -->
                <div class="flex-1 min-w-0">
                    <h2 style="font-size: 14px; font-weight: 700; color: var(--label-primary); letter-spacing: -0.02em; line-height: 1.2;">
                        {game.title}
                    </h2>
                    <div class="flex items-center gap-3 mt-1 num" style="font-size: 11px; color: var(--label-tertiary);">
                        <span class="flex items-center gap-1">
                            <HardDrive size={10} />
                            {formatSize(game.install_size)}
                        </span>
                        {#if game.total_playtime > 0}
                            <span class="flex items-center gap-1">
                                <Clock size={10} />
                                {formatPlaytime(game.total_playtime)}
                            </span>
                        {/if}
                        {#if game.last_played}
                            <span>{new Date(game.last_played).toLocaleDateString()}</span>
                        {/if}
                    </div>

                    <!-- Install path -->
                    {#if game.install_path}
                        <div class="flex items-center gap-1 mt-1 num"
                             style="font-size: 10px; color: var(--label-quaternary); min-width: 0;"
                             title={game.install_path}>
                            <FolderOpen size={9} style="flex-shrink: 0;" />
                            <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                {game.install_path}
                            </span>
                        </div>
                    {/if}

                    <!-- Exe selector -->
                    {#if showExeSelector && game.executables.length > 0}
                        <div class="mt-3 p-3 rounded-[8px]"
                             style="background: rgba(255,255,255,0.04); border: 1px solid var(--border-subtle);"
                             in:fly={{ y: 4, duration: 200 }}>
                            <div class="flex items-center justify-between mb-2">
                                <span style="font-size: 11px; font-weight: 500; color: var(--label-tertiary);">Select Executable</span>
                                <button
                                    on:click={() => handleRescan(game)}
                                    class="flex items-center gap-1 transition-colors"
                                    style="font-size: 11px; color: var(--label-tertiary);"
                                    on:mouseenter={e => (e.currentTarget as HTMLElement).style.color = 'var(--label-secondary)'}
                                    on:mouseleave={e => (e.currentTarget as HTMLElement).style.color = 'var(--label-tertiary)'}
                                >
                                    <RotateCcw size={10} /> Rescan
                                </button>
                            </div>
                            <div class="space-y-0.5 max-h-36 overflow-y-auto">
                                {#each game.executables.filter(e => e.exe_type !== 'installer' && e.exe_type !== 'redistributable') as exe}
                                    <button
                                        on:click={() => handleSetExecutable(game, exe.path)}
                                        class="w-full flex items-center gap-2 p-2 rounded-[6px] text-left transition-colors"
                                        style="border: 1px solid {game.primary_exe === exe.path ? 'var(--border)' : 'transparent'};
                                               background: {game.primary_exe === exe.path ? 'rgba(255,255,255,0.05)' : 'transparent'};"
                                        on:mouseenter={e => { if (game.primary_exe !== exe.path) (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.04)'; }}
                                        on:mouseleave={e => { if (game.primary_exe !== exe.path) (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
                                    >
                                        {#if game.primary_exe === exe.path}
                                            <Check size={10} style="color: var(--label-secondary); flex-shrink: 0;" />
                                        {:else}
                                            <div style="width: 10px; flex-shrink: 0;"></div>
                                        {/if}
                                        <div class="flex-1 min-w-0">
                                            <p style="font-size: 11px; color: var(--label-secondary);" class="truncate">{exe.name}.exe</p>
                                            <p style="font-size: 11px; color: var(--label-tertiary);" class="truncate num">{exe.path}</p>
                                        </div>
                                        {#if exe.exe_type !== 'unknown'}
                                            <span style="font-size: 11px; padding: 1px 5px; border-radius: 4px;
                                                         border: 1px solid var(--border-subtle); color: var(--label-tertiary);">
                                                {getExeTypeLabel(exe.exe_type)}
                                            </span>
                                        {/if}
                                    </button>
                                {/each}
                            </div>
                        </div>
                    {/if}
                </div>

                <!-- Action buttons -->
                <div class="flex items-center gap-2 shrink-0">
                    <!-- Play -->
                    <button
                        on:click={() => handleLaunch(game)}
                        disabled={isLaunching || !game.primary_exe}
                        class="flex items-center gap-2 transition-all hover:scale-[1.02] active:scale-[0.97] disabled:opacity-40 disabled:cursor-not-allowed"
                        style="padding: 9px 18px; background: #fff; color: #000;
                               font-size: 12px; font-weight: 700; letter-spacing: -0.01em;
                               border-radius: 8px; border: none;
                               box-shadow: 0 1px 3px rgba(0,0,0,0.4), inset 0 1px 0 rgba(255,255,255,0.3);"
                    >
                        {#if isLaunching}
                            <Loader2 size={13} class="animate-spin" />
                        {:else}
                            <Play size={13} fill="#000" />
                        {/if}
                        Play
                    </button>

                    <!-- Separator -->
                    <div style="width: 1px; height: 28px; background: var(--border-subtle);"></div>

                    <!-- Exe settings -->
                    <button
                        on:click={() => showExeSelector = !showExeSelector}
                        class="btn-icon"
                        style="width: 34px; height: 34px; border-radius: 8px;
                               {showExeSelector ? 'background: rgba(255,255,255,0.1); color: var(--label-primary);' : ''}"
                        title="Change executable"
                    >
                        <Settings2 size={14} />
                    </button>

                    <!-- Open folder -->
                    <button
                        on:click={() => handleOpenGameFolder(game)}
                        class="btn-icon"
                        style="width: 34px; height: 34px; border-radius: 8px;"
                        title="Open folder"
                    >
                        <FolderOpen size={14} />
                    </button>

                    <!-- Move to -->
                    <div style="position: relative;">
                        <button
                            on:click|stopPropagation={() => toggleMoveDropdown(game)}
                            disabled={isMoving}
                            class="btn-icon"
                            style="width: 34px; height: 34px; border-radius: 8px;
                                   {showMoveDropdown ? 'background: rgba(255,255,255,0.1); color: var(--label-primary);' : ''}"
                            title="Move to another library location"
                        >
                            {#if isMoving}
                                <Loader2 size={14} class="animate-spin" />
                            {:else}
                                <!-- two-files-transfer icon -->
                                <svg width="20" height="17" viewBox="0 0 22 16" fill="none"
                                     stroke="currentColor" stroke-width="1.4"
                                     stroke-linecap="round" stroke-linejoin="round">
                                    <!-- left file -->
                                    <path d="M1 1h4L7 3V13H1V1z"/>
                                    <path d="M5 1v2h2"/>
                                    <!-- arrow -->
                                    <path d="M9 7h4"/>
                                    <path d="M11.5 5l1.5 2-1.5 2"/>
                                    <!-- right file -->
                                    <path d="M15 1h4l2 2V13H15V1z"/>
                                    <path d="M19 1v2h2"/>
                                </svg>
                            {/if}
                        </button>

                        {#if showMoveDropdown}
                            <!-- svelte-ignore a11y_no_static_element_interactions -->
                            <div
                                on:click|stopPropagation
                                style="position: absolute; bottom: calc(100% + 8px); right: 0;
                                       min-width: 220px; z-index: 200;
                                       background: var(--bg-surface);
                                       border: 1px solid var(--border);
                                       border-radius: 10px;
                                       box-shadow: 0 8px 32px rgba(0,0,0,0.5);
                                       overflow: hidden;"
                                in:fly={{ y: 6, duration: 150 }}
                            >
                                <div style="padding: 8px 12px 6px; border-bottom: 1px solid var(--border-subtle);">
                                    <span style="font-size: 10px; font-weight: 600; letter-spacing: 0.08em;
                                                 text-transform: uppercase; color: var(--label-tertiary);">
                                        Move to Library
                                    </span>
                                </div>

                                {#if loadingLocations}
                                    <div class="flex items-center justify-center" style="padding: 14px;">
                                        <Loader2 size={14} class="animate-spin" style="color: var(--label-tertiary);" />
                                    </div>
                                {:else if knownLocations.length === 0}
                                    <div style="padding: 12px; font-size: 11px; color: var(--label-tertiary); text-align: center; line-height: 1.5;">
                                        No other library locations.<br>
                                        Add one in <strong style="color: var(--label-secondary);">Settings → Storage</strong>.
                                    </div>
                                {:else}
                                    {#each knownLocations as loc}
                                        <button
                                            on:click={() => handleMoveToLocation(game, loc)}
                                            class="w-full text-left"
                                            style="display: flex; flex-direction: column; gap: 2px;
                                                   padding: 9px 12px; border: none; background: transparent;
                                                   cursor: pointer; transition: background 0.12s;"
                                            on:mouseenter={e => (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.05)'}
                                            on:mouseleave={e => (e.currentTarget as HTMLElement).style.background = 'transparent'}
                                        >
                                            <span style="font-size: 12px; font-weight: 500; color: var(--label-primary);">
                                                {loc.label}
                                            </span>
                                            <span class="num" style="font-size: 10px; color: var(--label-tertiary);
                                                         white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 196px;">
                                                {loc.path}
                                            </span>
                                        </button>
                                    {/each}
                                {/if}

                                {#if !loadingLocations}
                                    <div style="border-top: 1px solid var(--border-subtle); padding: 5px 8px;">
                                        <button
                                            on:click={() => showMoveDropdown = false}
                                            class="w-full"
                                            style="font-size: 11px; color: var(--label-tertiary); background: transparent;
                                                   border: none; cursor: pointer; padding: 4px; border-radius: 5px;
                                                   transition: color 0.12s;"
                                            on:mouseenter={e => (e.currentTarget as HTMLElement).style.color = 'var(--label-secondary)'}
                                            on:mouseleave={e => (e.currentTarget as HTMLElement).style.color = 'var(--label-tertiary)'}
                                        >
                                            Cancel
                                        </button>
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>

                    <!-- Delete -->
                    <button
                        on:click={() => handleRemove(game)}
                        class="btn-icon transition-colors"
                        style="width: 34px; height: 34px; border-radius: 8px;"
                        title="Delete"
                        on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,69,58,0.15)'; (e.currentTarget as HTMLElement).style.color = '#ff453a'; (e.currentTarget as HTMLElement).style.borderColor = 'rgba(255,69,58,0.3)'; }}
                        on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = ''; (e.currentTarget as HTMLElement).style.color = ''; (e.currentTarget as HTMLElement).style.borderColor = ''; }}
                    >
                        <Trash2 size={14} />
                    </button>

                    <!-- Separator -->
                    <div style="width: 1px; height: 28px; background: var(--border-subtle);"></div>

                    <!-- Close -->
                    <button
                        on:click={() => selectedGame = null}
                        class="btn-icon"
                        style="width: 34px; height: 34px; border-radius: 8px;"
                    >
                        <X size={14} />
                    </button>
                </div>
            </div>
        </div>
    </div>
{/if}

<!-- Delete Confirmation Modal -->
{#if deleteTarget}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="fixed inset-0 z-[100] flex items-center justify-center"
        style="background: rgba(0,0,0,0.75); top: 32px;"
        in:fade={{ duration: 150 }}
        on:click={cancelDelete}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <div
            class="relative w-full mx-4"
            style="max-width: 360px;
                   background: var(--bg-surface);
                   border: 1px solid var(--border);
                   border-radius: 16px;
                   box-shadow: var(--shadow-dialog);
                   overflow: hidden;"
            in:scale={{ start: 0.95, duration: 220, easing: t => 1 - Math.pow(1 - t, 3) }}
            on:click|stopPropagation
            on:keydown|stopPropagation
        >
            <!-- Red top accent -->
            <div style="height: 2px; background: #ff453a; opacity: 0.7;"></div>

            <div class="p-5">
                <!-- Icon + title -->
                <div class="flex items-start gap-3 mb-4">
                    <div style="width: 36px; height: 36px; border-radius: 10px; flex-shrink: 0;
                                background: rgba(255,69,58,0.12); border: 1px solid rgba(255,69,58,0.25);
                                display: flex; align-items: center; justify-content: center;">
                        <AlertTriangle size={16} style="color: #ff453a;" />
                    </div>
                    <div>
                        <p style="font-size: 14px; font-weight: 700; color: var(--label-primary); letter-spacing: -0.02em;">Delete Game</p>
                        <p style="font-size: 11px; color: var(--label-tertiary); margin-top: 2px;">This action cannot be undone</p>
                    </div>
                </div>

                <!-- Game info box -->
                <div style="padding: 10px 12px; border-radius: 8px;
                             background: rgba(255,255,255,0.04); border: 1px solid var(--border-subtle);
                             margin-bottom: 16px;">
                    <p style="font-size: 12px; font-weight: 500; color: var(--label-primary);" class="truncate">
                        {deleteTarget.title}
                    </p>
                    {#if deleteTarget.install_size > 0}
                        <p class="flex items-center gap-1 num" style="font-size: 11px; color: var(--label-tertiary); margin-top: 3px;">
                            <HardDrive size={10} />
                            {formatSize(deleteTarget.install_size)}
                        </p>
                    {/if}
                </div>

                <p style="font-size: 11px; color: var(--label-tertiary); margin-bottom: 16px; line-height: 1.5;">
                    All files will be <span style="color: #ff453a; font-weight: 600;">permanently deleted</span> from disk.
                </p>

                <!-- Buttons -->
                <div class="flex gap-2">
                    <button
                        on:click={cancelDelete}
                        class="btn-secondary flex-1"
                        style="padding: 9px; font-size: 12px; border-radius: 8px; justify-content: center;"
                    >
                        Cancel
                    </button>
                    <button
                        on:click={confirmDelete}
                        disabled={isDeleting}
                        class="flex-1 flex items-center justify-center gap-1.5 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                        style="padding: 9px; font-size: 12px; font-weight: 700; border-radius: 8px;
                               background: #ff453a; color: #fff; border: none;
                               box-shadow: 0 1px 3px rgba(0,0,0,0.4);"
                        on:mouseenter={e => !isDeleting && ((e.currentTarget as HTMLElement).style.background = '#ff3024')}
                        on:mouseleave={e => (e.currentTarget as HTMLElement).style.background = '#ff453a'}
                    >
                        {#if isDeleting}
                            <Loader2 size={12} class="animate-spin" />
                            Deleting…
                        {:else}
                            <Trash2 size={12} />
                            Delete
                        {/if}
                    </button>
                </div>
            </div>
        </div>
    </div>
{/if}

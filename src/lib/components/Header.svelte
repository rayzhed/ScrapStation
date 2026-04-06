<script lang="ts">
    import { onMount } from 'svelte';
    import { animate } from 'motion';
    import { Zap, Layers, Library, Download, Settings, Gamepad2, Plus, FileCode2, CheckCircle2, XCircle, Sparkles, Info } from 'lucide-svelte';
    import * as LucideIcons from 'lucide-svelte';
    import { currentMode, navigateTo } from '$lib/stores/navigation';
    import { downloadStats } from '$lib/stores/downloads';
    import { libraryStats } from '$lib/stores/library';
    import { updateState } from '$lib/stores/updater';
    import { sources, currentSource } from '$lib/stores/sources';
    import { loadGames } from '$lib/stores/games';
    import { invoke } from '@tauri-apps/api/core';

    let sidebarEl: HTMLElement;
    let navEl: HTMLElement;

    // ── Drag-and-drop source install (Tauri native file drop) ────────────────
    // Tauri intercepts OS file drops before the browser sees them.
    // Must use onDragDropEvent from @tauri-apps/api/window.
    let isDraggingYaml = false;
    let dropFeedback: 'idle' | 'success' | 'error' = 'idle';
    let dropMessage = '';

    onMount(() => {
        // Register Tauri native drag-drop listener
        let unlistenDrop: (() => void) | null = null;
        import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
            getCurrentWindow().onDragDropEvent(async (event) => {
                const p = event.payload as any;
                if (p.type === 'enter' || p.type === 'over') {
                    isDraggingYaml = true;
                } else if (p.type === 'leave') {
                    isDraggingYaml = false;
                } else if (p.type === 'drop') {
                    isDraggingYaml = false;
                    const paths: string[] = (p.paths || []).filter((f: string) =>
                        f.endsWith('.yaml') || f.endsWith('.yml')
                    );
                    if (paths.length === 0) return;
                    let ok = 0, lastErr = '';
                    for (const path of paths) {
                        try {
                            const name: string = await invoke('install_source_from_path', { path });
                            ok++;
                            dropMessage = `"${name}" installed`;
                        } catch (e: any) { lastErr = e?.toString() || 'Error'; }
                    }
                    dropFeedback = ok > 0 ? 'success' : 'error';
                    if (ok > 1) dropMessage = `${ok} sources installed`;
                    if (ok === 0) dropMessage = lastErr;
                    setTimeout(() => { dropFeedback = 'idle'; dropMessage = ''; }, 3500);
                }
            }).then(fn => { unlistenDrop = fn; });
        });

        // Sidebar slides in from left
        animate(
            sidebarEl,
            { opacity: [0, 1], x: [-16, 0] },
            { duration: 0.40, easing: [0.22, 1, 0.36, 1] }
        );
        // Nav items stagger in
        const items = navEl.querySelectorAll('.sc-nav-item');
        items.forEach((el, i) => {
            (el as HTMLElement).style.opacity = '0';
            animate(
                el as HTMLElement,
                { opacity: [0, 1], x: [-8, 0] },
                { delay: 0.12 + i * 0.055, duration: 0.32, easing: [0.22, 1, 0.36, 1] }
            );
        });

        return () => { unlistenDrop?.(); };
    });

    function getIconComponent(iconName: string) {
        const pascal = iconName.split('-').map(w => w[0].toUpperCase() + w.slice(1)).join('');
        return (LucideIcons as any)[pascal] || LucideIcons.Globe;
    }

    async function selectSource(id: string) {
        currentSource.set(id);
        await loadGames(id, 1);
    }

    const navItems = [
        { mode: 'updates'   as const, label: 'Updates',   icon: Sparkles },
        { mode: 'browse'    as const, label: 'Browse',    icon: Gamepad2 },
        { mode: 'library'   as const, label: 'Library',   icon: Library  },
        { mode: 'downloads' as const, label: 'Downloads', icon: Download },
    ];

    // Svelte action: stagger source items in as they mount
    function sourceIn(node: HTMLElement, index: number) {
        node.style.opacity = '0';
        animate(
            node,
            { opacity: [0, 1], x: [-6, 0] },
            { delay: 0.05 * index, duration: 0.26, easing: [0.22, 1, 0.36, 1] }
        );
    }
</script>

<aside
    bind:this={sidebarEl}
    class="flex flex-col shrink-0 h-full"
    style="
        width: 200px;
        background: var(--bg-sidebar);
        border-right: 1px solid var(--border-subtle);
    "
>
    <!-- Logo -->
    <div
        class="flex items-center gap-2.5 px-4 shrink-0"
        style="height: 44px; border-bottom: 1px solid rgba(255,255,255,0.18);"
    >
        <Zap size={13} strokeWidth={2} style="color: var(--label-tertiary); flex-shrink: 0;" />
        <span style="font-size: 12px; font-weight: 700; letter-spacing: 0.07em; color: var(--label-primary);">
            SCRAPSTATION
        </span>
    </div>

    <!-- Navigation -->
    <nav bind:this={navEl} style="border-bottom: 1px solid rgba(255,255,255,0.18);">
        {#each navItems as item}
            {@const isActive = $currentMode === item.mode}
            <button
                on:click={() => navigateTo(item.mode)}
                class="sc-nav-item {isActive ? 'active' : ''}"
            >
                <svelte:component
                    this={item.icon}
                    size={14}
                    strokeWidth={isActive ? 2 : 1.75}
                    style="flex-shrink: 0; color: {isActive ? 'var(--label-primary)' : 'var(--label-tertiary)'};"
                />
                <span style="flex: 1;">{item.label}</span>

                {#if item.mode === 'updates' && ($updateState.phase === 'available' || $updateState.phase === 'ready')}
                    {@const dotColor = $updateState.phase === 'ready' ? '#32d74b' : '#0a84ff'}
                    <span class="relative flex flex-shrink-0" style="width: 10px; height: 10px;">
                        <span class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-60"
                              style="background: {dotColor};"></span>
                        <span class="relative inline-flex rounded-full m-auto"
                              style="width: 6px; height: 6px; background: {dotColor};"></span>
                    </span>
                {:else if item.mode === 'library' && $libraryStats.gameCount > 0}
                    <span class="sc-badge">{$libraryStats.gameCount}</span>
                {:else if item.mode === 'downloads' && $downloadStats.pendingCount > 0}
                    <span class="sc-badge" style="
                        background: rgba(10,132,255,0.15);
                        color: #0a84ff;
                        border-color: rgba(10,132,255,0.25);
                    ">{$downloadStats.pendingCount}</span>
                {/if}
            </button>
        {/each}
    </nav>

    <!-- Sources -->
    <div class="flex-1 overflow-y-auto">
        {#if $currentMode === 'browse'}
            <p style="
                font-size: 11px; font-weight: 600;
                letter-spacing: 0.06em; text-transform: uppercase;
                color: var(--label-quaternary);
                padding: 9px 12px 5px;
            ">
                Sources
            </p>

            <div class="flex flex-col">
                {#each $sources as source, i}
                    {@const isActive = $currentSource === source.id}
                    {@const Icon = getIconComponent(source.icon || 'globe')}
                    <button
                        use:sourceIn={i}
                        on:click={() => selectSource(source.id)}
                        class="sc-nav-item"
                        style="{isActive ? 'background: rgba(255,255,255,0.08); color: var(--label-primary);' : ''}"
                    >
                        <!-- Status dot -->
                        <span
                            style="
                                flex-shrink: 0;
                                width: 6px; height: 6px;
                                border-radius: 50%;
                                background: {source.color};
                                opacity: {isActive ? '1' : '0.5'};
                            "
                        ></span>
                        <svelte:component
                            this={Icon}
                            size={13}
                            strokeWidth={1.75}
                            style="flex-shrink: 0; color: {isActive ? source.color : 'var(--label-tertiary)'};"
                        />
                        <span class="truncate" style="
                            color: {isActive ? 'var(--label-primary)' : 'var(--label-secondary)'};
                        ">{source.name}</span>
                    </button>
                {/each}
            </div>
        {/if}
    </div>

    <!-- Add source / drop feedback -->
    <div style="border-top: 1px solid rgba(255,255,255,0.18); border-bottom: 1px solid rgba(255,255,255,0.18);">
        {#if dropFeedback === 'success'}
            <div class="sc-nav-item" style="color: #32d74b; cursor: default;">
                <CheckCircle2 size={14} strokeWidth={2} style="flex-shrink: 0; color: #32d74b;" />
                <span class="truncate">{dropMessage}</span>
            </div>
        {:else if dropFeedback === 'error'}
            <div class="sc-nav-item" style="color: #ff453a; cursor: default;">
                <XCircle size={14} strokeWidth={2} style="flex-shrink: 0; color: #ff453a;" />
                <span class="truncate">{dropMessage}</span>
            </div>
        {:else}
            <div class="sc-nav-item" style="color: var(--label-quaternary); cursor: default; border-style: dashed;">
                <Plus size={14} strokeWidth={1.75} style="flex-shrink: 0;" />
                <span>Drop .yaml to add</span>
            </div>
        {/if}
    </div>

    <!-- Settings + About -->
    <div style="border-top: 1px solid rgba(255,255,255,0.18);">
        <button on:click={() => navigateTo('settings')} class="sc-nav-item {$currentMode === 'settings' ? 'active' : ''}">
            <Settings size={14} strokeWidth={$currentMode === 'settings' ? 2 : 1.75} style="flex-shrink: 0; color: {$currentMode === 'settings' ? 'var(--label-primary)' : 'var(--label-tertiary)'};" />
            <span>Settings</span>
        </button>
        <button on:click={() => navigateTo('about')} class="sc-nav-item {$currentMode === 'about' ? 'active' : ''}">
            <Info size={14} strokeWidth={$currentMode === 'about' ? 2 : 1.75} style="flex-shrink: 0; color: {$currentMode === 'about' ? 'var(--label-primary)' : 'var(--label-tertiary)'};" />
            <span>About</span>
        </button>
    </div>
</aside>

<!-- Full-app drag overlay -->
{#if isDraggingYaml}
    <div
        class="fixed inset-0 z-[9999] flex flex-col items-center justify-center pointer-events-none"
        style="background: rgba(0,0,0,0.55); backdrop-filter: blur(6px);"
    >
        <div
            class="flex flex-col items-center gap-4 p-10 rounded-[16px]"
            style="border: 2px dashed rgba(255,255,255,0.25); background: rgba(255,255,255,0.04);"
        >
            <FileCode2 size={40} strokeWidth={1.5} style="color: rgba(255,255,255,0.5);" />
            <p style="font-size: 15px; font-weight: 600; color: var(--label-primary); letter-spacing: -0.01em;">
                Drop YAML config to install source
            </p>
            <p style="font-size: 12px; color: var(--label-tertiary);">
                .yaml or .yml files only
            </p>
        </div>
    </div>
{/if}


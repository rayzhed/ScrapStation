<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { RefreshCw, Download, ExternalLink, CheckCircle, Sparkles } from 'lucide-svelte';
    import { updateState, checkForUpdates, installUpdate, applyUpdate } from '$lib/stores/updater';
    import { invoke } from '@tauri-apps/api/core';
    import { listen, type UnlistenFn } from '@tauri-apps/api/event';

    type Channel = 'stable' | 'dev';

    interface GithubAsset {
        name: string;
        browser_download_url: string;
    }

    interface GithubRelease {
        tag_name: string;
        name: string;
        body: string;
        published_at: string;
        prerelease: boolean;
        html_url: string;
        assets: GithubAsset[];
    }

    interface InstallerProgress {
        downloaded: number;
        total: number | null;
        percent: number | null;
    }

    let channel: Channel = 'stable';
    let releases: GithubRelease[] = [];
    let loadingReleases = false;
    let fetchError = '';

    // Track which release tag is currently being downloaded in-app
    let installingTag: string | null = null;
    let installerPercent: number | null = null;
    let installerLaunching = false;

    let unlistenProgress: UnlistenFn | null = null;

    const APP_VERSION = '0.1.0';
    const REPO = 'rayzhed/ScrapStation';

    async function fetchReleases() {
        loadingReleases = true;
        fetchError = '';
        try {
            const res = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=20`);
            if (!res.ok) throw new Error(`GitHub API error: ${res.status}`);
            const all: GithubRelease[] = await res.json();
            releases = channel === 'stable'
                ? all.filter(r => !r.prerelease)
                : all.filter(r => r.prerelease);
        } catch (e) {
            fetchError = String(e);
        } finally {
            loadingReleases = false;
        }
    }

    function switchChannel(c: Channel) {
        channel = c;
        fetchReleases();
    }

    function formatDate(iso: string): string {
        return new Date(iso).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' });
    }

    function openInBrowser(url: string) {
        invoke('open_url_in_browser', { url }).catch(() => {});
    }

    /** Find the NSIS .exe installer in a release's assets. Returns null if none found. */
    function getInstallerAsset(release: GithubRelease): GithubAsset | null {
        return release.assets.find(a =>
            a.name.endsWith('.exe') || a.name.endsWith('_x64-setup.exe') || a.name.includes('setup')
        ) ?? null;
    }

    async function installVersion(release: GithubRelease) {
        // Use tauri-plugin-updater for the latest detected stable update
        const isLatestUpdate = channel === 'stable'
            && $updateState.phase === 'available'
            && ($updateState.version === release.tag_name || `v${$updateState.version}` === release.tag_name);

        if (isLatestUpdate) {
            installUpdate();
            return;
        }

        // For everything else, download the installer directly in-app
        const asset = getInstallerAsset(release);
        if (!asset) {
            // No installer asset found — fall back to browser
            openInBrowser(release.html_url);
            return;
        }

        if (installingTag) return; // already downloading something

        installingTag = release.tag_name;
        installerPercent = 0;
        installerLaunching = false;

        // Listen for progress events
        unlistenProgress = await listen<InstallerProgress>('installer-progress', (event) => {
            installerPercent = event.payload.percent ?? null;
        });

        try {
            await invoke('download_and_run_installer', { url: asset.browser_download_url });
            // After invoke resolves the app exits — but set launching state just in case
            installerLaunching = true;
        } catch (e) {
            installingTag = null;
            installerPercent = null;
            installerLaunching = false;
        } finally {
            unlistenProgress?.();
            unlistenProgress = null;
        }
    }

    onMount(() => {
        fetchReleases();
    });

    onDestroy(() => {
        unlistenProgress?.();
    });
</script>

<div class="flex flex-col h-full">
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-5 border-b"
         style="border-color: var(--border-subtle); flex-shrink: 0;">
        <div>
            <h1 class="text-sm font-bold tracking-wide" style="color: var(--label-primary); letter-spacing: 0.06em;">
                UPDATES
            </h1>
            <p class="text-[11px] mt-0.5" style="color: var(--label-tertiary);">
                Current version: <span class="font-mono">v{APP_VERSION}</span>
            </p>
        </div>

        <!-- Channel toggle -->
        <div class="flex items-center p-0.5 rounded-[8px]"
             style="background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.08);">
            {#each (['stable', 'dev'] as Channel[]) as ch}
                <button
                    on:click={() => switchChannel(ch)}
                    class="px-3 py-1 text-[11px] font-medium rounded-[6px] transition-all"
                    style={channel === ch
                        ? 'background: rgba(255,255,255,0.1); color: var(--label-primary);'
                        : 'color: var(--label-tertiary);'}
                >
                    {ch === 'stable' ? 'Stable' : 'Dev'}
                </button>
            {/each}
        </div>
    </div>

    <!-- Update action banner (stable channel only) -->
    {#if channel === 'stable'}
        <div class="px-6 pt-4 flex-shrink-0">
            {#if $updateState.phase === 'available'}
                <div class="flex items-center justify-between p-3 rounded-[10px]"
                     style="background: rgba(10,132,255,0.08); border: 1px solid rgba(10,132,255,0.25);">
                    <div class="flex items-center gap-2.5">
                        <Sparkles size={14} style="color: #0a84ff;" />
                        <div>
                            <p class="text-xs font-semibold" style="color: #0a84ff;">
                                v{$updateState.version} is available
                            </p>
                            <p class="text-[11px] mt-0.5" style="color: var(--label-tertiary);">
                                You're on v{APP_VERSION}
                            </p>
                        </div>
                    </div>
                    <button
                        on:click={installUpdate}
                        class="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[7px] transition-colors"
                        style="background: #0a84ff; color: white;"
                    >
                        <Download size={12} /> Install
                    </button>
                </div>
            {:else if $updateState.phase === 'checking'}
                <div class="flex items-center gap-2 p-3 rounded-[10px]"
                     style="background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08);">
                    <div class="w-3.5 h-3.5 rounded-full animate-spin flex-shrink-0"
                         style="border: 1.5px solid rgba(255,255,255,0.1); border-top-color: rgba(255,255,255,0.5);"></div>
                    <span class="text-[11px]" style="color: var(--label-tertiary);">Checking for updates…</span>
                </div>
            {:else if $updateState.phase === 'downloading'}
                <div class="p-3 rounded-[10px]"
                     style="background: rgba(10,132,255,0.08); border: 1px solid rgba(10,132,255,0.25);">
                    <div class="flex items-center justify-between mb-2">
                        <span class="text-[11px] font-medium" style="color: #0a84ff;">
                            Downloading v{$updateState.version}…
                        </span>
                        <span class="text-[11px] font-mono" style="color: var(--label-tertiary);">
                            {$updateState.progress ?? 0}%
                        </span>
                    </div>
                    <div class="h-1 rounded-full overflow-hidden" style="background: rgba(255,255,255,0.06);">
                        <div class="h-full rounded-full transition-all"
                             style="width: {$updateState.progress ?? 0}%; background: #0a84ff;"></div>
                    </div>
                </div>
            {:else if $updateState.phase === 'ready'}
                <div class="flex items-center justify-between p-3 rounded-[10px]"
                     style="background: rgba(50,215,75,0.08); border: 1px solid rgba(50,215,75,0.25);">
                    <div class="flex items-center gap-2.5">
                        <CheckCircle size={14} style="color: #32d74b;" />
                        <p class="text-xs font-medium" style="color: #32d74b;">
                            Update ready — restart to apply
                        </p>
                    </div>
                    <button
                        on:click={applyUpdate}
                        class="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[7px]"
                        style="background: #32d74b; color: black;"
                    >
                        <RefreshCw size={12} /> Restart
                    </button>
                </div>
            {:else}
                <div class="flex items-center justify-between p-3 rounded-[10px]"
                     style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06);">
                    <div class="flex items-center gap-2">
                        <CheckCircle size={13} style="color: var(--label-quaternary);" />
                        <span class="text-[11px]" style="color: var(--label-tertiary);">
                            {$updateState.phase === 'error' ? ($updateState.error ?? 'Update check failed') : 'Up to date'}
                        </span>
                    </div>
                    <button
                        on:click={checkForUpdates}
                        class="flex items-center gap-1.5 px-2.5 py-1 text-[11px] rounded-[6px] border transition-colors hover:bg-white/5"
                        style="border-color: rgba(255,255,255,0.1); color: var(--label-secondary);"
                    >
                        <RefreshCw size={10} /> Check
                    </button>
                </div>
            {/if}
        </div>
    {/if}

    <!-- Release list -->
    <div class="flex-1 overflow-y-auto px-6 py-4 space-y-3">
        {#if loadingReleases}
            <div class="flex items-center justify-center py-16">
                <div class="w-5 h-5 rounded-full animate-spin"
                     style="border: 1.5px solid rgba(255,255,255,0.1); border-top-color: rgba(255,255,255,0.4);"></div>
            </div>
        {:else if fetchError}
            <div class="flex flex-col items-center justify-center py-16 text-center gap-2">
                <p class="text-[12px]" style="color: var(--label-tertiary);">Could not load releases</p>
                <p class="text-[11px] font-mono" style="color: var(--label-quaternary);">{fetchError}</p>
                <button
                    on:click={fetchReleases}
                    class="mt-2 px-3 py-1.5 text-[11px] rounded-[6px] border transition-colors hover:bg-white/5"
                    style="border-color: rgba(255,255,255,0.1); color: var(--label-secondary);"
                >
                    Retry
                </button>
            </div>
        {:else if releases.length === 0}
            <div class="flex flex-col items-center justify-center py-16 text-center">
                <p class="text-[12px]" style="color: var(--label-tertiary);">No {channel} releases yet</p>
            </div>
        {:else}
            {#each releases as release}
                {@const isCurrent = release.tag_name === `v${APP_VERSION}` || release.tag_name === APP_VERSION}
                {@const isAvailable = channel === 'stable' && $updateState.phase === 'available' && $updateState.version && (release.tag_name === `v${$updateState.version}` || release.tag_name === $updateState.version)}
                <div class="rounded-[10px] p-4 transition-colors"
                     style="
                        background: rgba(255,255,255,0.03);
                        border: 1px solid {isAvailable ? 'rgba(10,132,255,0.3)' : isCurrent ? 'rgba(50,215,75,0.2)' : 'rgba(255,255,255,0.07)'};
                     ">
                    <div class="flex items-start justify-between gap-3 mb-2">
                        <div class="flex items-center gap-2 flex-wrap">
                            <span class="text-xs font-bold font-mono" style="color: var(--label-primary);">
                                {release.name || release.tag_name}
                            </span>
                            {#if isCurrent}
                                <span class="px-1.5 py-0.5 text-[10px] font-medium rounded-[4px]"
                                      style="background: rgba(50,215,75,0.12); color: #32d74b; border: 1px solid rgba(50,215,75,0.2);">
                                    installed
                                </span>
                            {:else if isAvailable}
                                <span class="px-1.5 py-0.5 text-[10px] font-medium rounded-[4px]"
                                      style="background: rgba(10,132,255,0.12); color: #0a84ff; border: 1px solid rgba(10,132,255,0.2);">
                                    available
                                </span>
                            {/if}
                            {#if release.prerelease}
                                <span class="px-1.5 py-0.5 text-[10px] font-medium rounded-[4px]"
                                      style="background: rgba(255,159,10,0.1); color: #ff9f0a; border: 1px solid rgba(255,159,10,0.2);">
                                    pre-release
                                </span>
                            {/if}
                        </div>
                        <div class="flex items-center gap-2 flex-shrink-0">
                            <span class="text-[11px]" style="color: var(--label-quaternary);">
                                {formatDate(release.published_at)}
                            </span>
                            <button
                                on:click={() => openInBrowser(release.html_url)}
                                class="p-1 rounded-[5px] hover:bg-white/8 transition-colors"
                                style="color: var(--label-quaternary);"
                                title="Open on GitHub"
                            >
                                <ExternalLink size={11} />
                            </button>
                        </div>
                    </div>

                    {#if release.body}
                        <p class="text-[11px] leading-relaxed whitespace-pre-line line-clamp-6"
                           style="color: var(--label-tertiary);">
                            {release.body}
                        </p>
                    {/if}

                    {#if !isCurrent}
                        {@const isDownloadingThis = installingTag === release.tag_name}
                        <div class="mt-3 pt-3" style="border-top: 1px solid rgba(255,255,255,0.06);">
                            {#if isDownloadingThis}
                                <!-- Download progress -->
                                <div class="flex flex-col gap-1.5">
                                    <div class="flex items-center justify-between">
                                        <span class="text-[11px]" style="color: var(--label-tertiary);">
                                            {installerLaunching ? 'Launching installer…' : `Downloading… ${installerPercent ?? 0}%`}
                                        </span>
                                        <span class="text-[11px] font-mono" style="color: var(--label-quaternary);">
                                            {installerPercent ?? 0}%
                                        </span>
                                    </div>
                                    <div class="h-1 rounded-full overflow-hidden" style="background: rgba(255,255,255,0.06);">
                                        <div class="h-full rounded-full transition-all"
                                             style="width: {installerPercent ?? 0}%; background: {isAvailable ? '#0a84ff' : 'rgba(255,255,255,0.4)'};">
                                        </div>
                                    </div>
                                </div>
                            {:else}
                                <div class="flex items-center justify-between">
                                    <span class="text-[11px]" style="color: var(--label-quaternary);">
                                        {isAvailable ? 'Newer version' : 'Older version'} — installer will replace current
                                    </span>
                                    <button
                                        on:click={() => installVersion(release)}
                                        disabled={!!installingTag}
                                        class="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[7px] transition-colors disabled:opacity-40"
                                        style={isAvailable
                                            ? 'background: #0a84ff; color: white;'
                                            : 'background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.12); color: var(--label-secondary);'}
                                    >
                                        <Download size={11} />
                                        {isAvailable ? 'Install update' : 'Install this version'}
                                    </button>
                                </div>
                            {/if}
                        </div>
                    {/if}
                </div>
            {/each}
        {/if}
    </div>
</div>

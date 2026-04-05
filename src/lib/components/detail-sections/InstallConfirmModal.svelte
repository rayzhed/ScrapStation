<script lang="ts">
    import { HardDrive, FolderOpen, Download, AlertTriangle, XCircle, CheckCircle, ChevronDown, X } from 'lucide-svelte';

    export let gameTitle: string = '';
    export let coverUrl: string = '';

    export let preflight: {
        download_size_bytes: number;
        download_path: string;
        install_path: string;
        available_bytes: number;
    } | null = null;

    export let loading: boolean = false;
    export let sizeProbing: boolean = false;
    export let onConfirm: () => void;
    export let onCancel: () => void;
    /** Called with a path string when user selects a location, or null when they choose "Browse…" */
    export let onChangeDownloadDir: ((path: string | null) => void) | null = null;
    /** Called with a path string when user selects a location, or null when they choose "Browse…" */
    export let onChangeInstallDir: ((path: string | null) => void) | null = null;
    /** Known download directory options to show in the picker dropdown */
    export let downloadDirOptions: { path: string; label: string }[] = [];
    /** Known install directory options to show in the picker dropdown */
    export let installDirOptions: { path: string; label: string }[] = [];

    let showDownloadPicker = false;
    let showInstallPicker = false;

    const COMPRESSION_RATIO = 2.5;

    let advancedOpen = false;

    function formatBytes(bytes: number): string {
        if (bytes <= 0) return 'Unknown';
        if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(1) + ' GB';
        if (bytes >= 1_048_576)     return (bytes / 1_048_576).toFixed(1) + ' MB';
        return (bytes / 1024).toFixed(0) + ' KB';
    }

    function shortenPath(p: string): string {
        if (!p) return '';
        const sep = p.includes('\\') ? '\\' : '/';
        const parts = p.split(sep).filter(Boolean);
        if (parts.length <= 4) return p;
        return parts[0] + sep + '...' + sep + parts.slice(-2).join(sep);
    }

    $: downloadSize     = preflight?.download_size_bytes ?? 0;
    $: available        = preflight?.available_bytes ?? 0;
    $: downloadPath     = preflight?.download_path ?? '';
    $: installPath      = preflight?.install_path ?? '';
    $: estimatedInstall = downloadSize > 0 ? Math.round(downloadSize * COMPRESSION_RATIO) : 0;
    $: totalRequired    = downloadSize > 0 ? downloadSize + estimatedInstall : 0;
    $: sizeKnown        = downloadSize > 0 && available > 0;
    $: notEnoughToDl    = sizeKnown && available < downloadSize;
    $: notEnoughToInstall = sizeKnown && !notEnoughToDl && available < totalRequired;
    $: canInstall       = !loading && !sizeProbing;
    $: drive            = downloadPath ? downloadPath.split(/[\\/]/)[0] : '';
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="fixed inset-0 z-[200] flex items-center justify-center"
    style="background: rgba(0,0,0,0.8);"
    on:click={onCancel}
    on:keydown={(e) => e.key === 'Escape' && onCancel()}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
>
    <div
        class="relative w-full max-w-2xl mx-4 flex flex-col"
        style="background: var(--bg-surface); border: 1px solid var(--border); border-radius: 12px; box-shadow: var(--shadow-dialog); max-height: 90vh;"
        on:click|stopPropagation
        on:keydown|stopPropagation
    >
        <!-- Header -->
        <div class="flex items-start justify-between px-7 pt-6 pb-5" style="border-bottom: 1px solid var(--border-subtle);">
            <div class="flex items-center gap-3">
                <div class="w-11 h-11 rounded-[8px] flex items-center justify-center"
                     style="border: 1px solid var(--border); background: rgba(255,255,255,0.04);">
                    <Download size={18} style="color: var(--label-tertiary);" />
                </div>
                <div>
                    <h2 class="text-[15px] font-semibold" style="color: var(--label-primary); letter-spacing: -0.01em;">Install Game</h2>
                    <p class="text-[13px] mt-0.5 truncate" style="color: var(--label-tertiary); max-width: 420px;">{gameTitle}</p>
                </div>
            </div>
            <button
                on:click={onCancel}
                class="p-2 rounded-[6px] transition-colors"
                style="color: var(--label-tertiary);"
                on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.07)'; (e.currentTarget as HTMLElement).style.color = 'var(--label-primary)'; }}
                on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'transparent'; (e.currentTarget as HTMLElement).style.color = 'var(--label-tertiary)'; }}
            >
                <X size={16} />
            </button>
        </div>

        <div style="overflow-y: {showDownloadPicker || showInstallPicker ? 'visible' : 'auto'}; flex: 1; min-height: 0;">
        {#if loading}
            <div class="px-7 py-12 flex flex-col items-center gap-3">
                <div class="w-7 h-7 rounded-full animate-spin"
                     style="border: 2px solid rgba(255,255,255,0.1); border-top-color: rgba(255,255,255,0.5);"></div>
                <p class="text-[13px]" style="color: var(--label-tertiary);">Checking disk space…</p>
            </div>

        {:else if preflight}
            <!-- Save to row -->
            <div class="px-7 pt-5 pb-4">
                <p class="text-[11px] font-semibold uppercase mb-2" style="letter-spacing: 0.07em; color: var(--label-tertiary);">Save archive to</p>
                <div class="flex items-center gap-3 px-4 py-3 rounded-[8px]"
                     style="background: var(--bg-raised); border: 1px solid var(--border-subtle);">
                    <FolderOpen size={15} style="color: var(--label-tertiary); flex-shrink: 0;" />
                    <p class="text-[13px] font-mono truncate flex-1 min-w-0" style="color: var(--label-secondary);" title={downloadPath}>
                        {downloadPath || '…'}
                    </p>
                    {#if onChangeDownloadDir}
                        <div style="position: relative; flex-shrink: 0;">
                            <button
                                on:click|stopPropagation={() => { showDownloadPicker = !showDownloadPicker; showInstallPicker = false; }}
                                class="flex-shrink-0 text-[12px] font-medium px-3 py-1.5 rounded-[5px] border border-white/10 hover:bg-white/8 transition-colors"
                                style="color: var(--label-secondary);"
                            >Change</button>
                            {#if showDownloadPicker}
                                <div class="loc-dropdown">
                                    {#if downloadDirOptions.length > 0}
                                        <div class="loc-header">Download to</div>
                                        {#each downloadDirOptions as opt}
                                            <button class="loc-option" on:click|stopPropagation={() => { showDownloadPicker = false; onChangeDownloadDir && onChangeDownloadDir(opt.path); }}>
                                                <span class="loc-path">{opt.path}</span>
                                                <span class="loc-label">{opt.label}</span>
                                            </button>
                                        {/each}
                                        <div class="loc-divider"></div>
                                    {/if}
                                    <button class="loc-option" on:click|stopPropagation={() => { showDownloadPicker = false; onChangeDownloadDir && onChangeDownloadDir(null); }}>
                                        <span class="loc-path" style="display: flex; align-items: center; gap: 5px;">
                                            <FolderOpen size={11} /> Browse for custom folder…
                                        </span>
                                    </button>
                                </div>
                            {/if}
                        </div>
                    {/if}
                </div>
            </div>

            <!-- Disk space table -->
            <div class="px-7 pb-4">
                <p class="text-[11px] font-semibold uppercase mb-2" style="letter-spacing: 0.07em; color: var(--label-tertiary);">Disk Space</p>
                <div class="rounded-[8px] overflow-hidden" style="border: 1px solid var(--border-subtle);">
                    <table class="w-full">
                        <tbody>
                            <tr style="border-bottom: 1px solid var(--border-subtle);">
                                <td class="px-4 py-3 text-[13px] flex items-center gap-2" style="color: var(--label-tertiary);">
                                    <Download size={13} class="shrink-0" />
                                    Download size
                                </td>
                                <td class="px-4 py-3 text-right text-[13px] font-medium" style="color: var(--label-secondary);">
                                    {#if sizeProbing && downloadSize === 0}
                                        <span class="inline-flex items-center gap-2" style="color: var(--label-tertiary);">
                                            <span class="inline-block w-3.5 h-3.5 rounded-full animate-spin" style="border: 1.5px solid rgba(255,255,255,0.1); border-top-color: rgba(255,255,255,0.5);"></span>
                                            Probing…
                                        </span>
                                    {:else}
                                        {downloadSize > 0 ? formatBytes(downloadSize) : 'Unknown'}
                                    {/if}
                                </td>
                            </tr>

                            {#if estimatedInstall > 0}
                                <tr style="border-bottom: 1px solid var(--border-subtle);">
                                    <td class="px-4 py-3 text-[13px] flex items-center gap-2" style="color: var(--label-tertiary);">
                                        <HardDrive size={13} class="shrink-0" />
                                        Estimated installed
                                    </td>
                                    <td class="px-4 py-3 text-right text-[13px]" style="color: var(--label-tertiary);">
                                        ~{formatBytes(estimatedInstall)}
                                    </td>
                                </tr>

                                <tr style="border-bottom: 1px solid var(--border-subtle); background: rgba(255,255,255,0.02);">
                                    <td class="px-4 py-3 text-[13px] font-medium" style="color: var(--label-secondary);">Total required</td>
                                    <td class="px-4 py-3 text-right text-[13px] font-bold" style="color: var(--label-primary);">
                                        ~{formatBytes(totalRequired)}
                                    </td>
                                </tr>
                            {/if}

                            <tr>
                                <td class="px-4 py-3 text-[13px] font-medium flex items-center gap-2"
                                    style="color: {notEnoughToDl ? '#ff453a' : notEnoughToInstall ? '#ff9f0a' : 'var(--label-secondary)'};">
                                    {#if notEnoughToDl}
                                        <XCircle size={13} class="shrink-0" />
                                    {:else if notEnoughToInstall}
                                        <AlertTriangle size={13} class="shrink-0" />
                                    {:else}
                                        <CheckCircle size={13} class="shrink-0" style="color: #32d74b;" />
                                    {/if}
                                    Available ({drive})
                                </td>
                                <td class="px-4 py-3 text-right text-[13px] font-bold"
                                    style="color: {notEnoughToDl ? '#ff453a' : notEnoughToInstall ? '#ff9f0a' : '#32d74b'};">
                                    {available > 0 ? formatBytes(available) : 'Unknown'}
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- Warning -->
            {#if notEnoughToDl}
                <div class="mx-7 mb-4 flex items-start gap-3 px-4 py-3 rounded-[8px]"
                     style="border: 1px solid rgba(255,69,58,0.25);">
                    <XCircle size={15} style="color: #ff453a; flex-shrink: 0; margin-top: 1px;" />
                    <p class="text-[13px] leading-relaxed" style="color: rgba(255,69,58,0.85);">
                        <span style="font-weight: 700; color: #ff453a;">Not enough space.</span>
                        {#if totalRequired > 0}
                            Requires ~{formatBytes(totalRequired)}, only {formatBytes(available)} available on {drive}.
                        {:else}
                            Free up space on {drive} and try again.
                        {/if}
                    </p>
                </div>
            {:else if notEnoughToInstall}
                <div class="mx-7 mb-4 flex items-start gap-3 px-4 py-3 rounded-[8px]"
                     style="border: 1px solid rgba(255,159,10,0.25);">
                    <AlertTriangle size={15} style="color: #ff9f0a; flex-shrink: 0; margin-top: 1px;" />
                    <p class="text-[13px] leading-relaxed" style="color: rgba(255,159,10,0.85);">
                        You may not have enough space to extract. Consider freeing up space on {drive}.
                    </p>
                </div>
            {:else if !sizeKnown}
                <div class="mx-7 mb-4 flex items-start gap-3 px-4 py-3 rounded-[8px]"
                     style="border: 1px solid var(--border-subtle);">
                    <AlertTriangle size={15} style="color: var(--label-tertiary); flex-shrink: 0; margin-top: 1px;" />
                    <p class="text-[13px]" style="color: var(--label-tertiary);">
                        Could not determine file size. Ensure you have enough free space.
                    </p>
                </div>
            {/if}

            <!-- Advanced: install location -->
            <div class="mx-7 mb-4" style="border: 1px solid var(--border-subtle); border-radius: 8px;">
                <button
                    class="w-full flex items-center justify-between px-4 py-2.5 hover:bg-white/4 transition-colors"
                    style="border-radius: {advancedOpen ? '8px 8px 0 0' : '8px'};"
                    on:click={() => advancedOpen = !advancedOpen}
                >
                    <span class="text-[13px]" style="color: var(--label-tertiary);">Advanced</span>
                    <ChevronDown size={14} style="color: var(--label-quaternary); transition: transform 0.15s; transform: rotate({advancedOpen ? 180 : 0}deg);" />
                </button>
                {#if advancedOpen}
                    <div class="px-4 pb-4 pt-2" style="border-top: 1px solid var(--border-subtle);">
                        <p class="text-[11px] font-semibold uppercase mb-2" style="letter-spacing: 0.07em; color: var(--label-quaternary);">Install Location</p>
                        <div class="flex items-center gap-3 px-3 py-2.5 rounded-[6px]"
                             style="background: var(--bg-raised); border: 1px solid var(--border-subtle);">
                            <FolderOpen size={14} style="color: var(--label-quaternary); flex-shrink: 0;" />
                            <p class="text-[12px] font-mono truncate flex-1 min-w-0" style="color: var(--label-tertiary);" title={installPath}>
                                {installPath || '…'}
                            </p>
                            {#if onChangeInstallDir}
                                <div style="position: relative; flex-shrink: 0;">
                                    <button
                                        on:click|stopPropagation={() => { showInstallPicker = !showInstallPicker; showDownloadPicker = false; }}
                                        class="flex-shrink-0 text-[12px] font-medium px-3 py-1.5 rounded-[5px] border border-white/10 hover:bg-white/8 transition-colors"
                                        style="color: var(--label-tertiary);"
                                    >Change</button>
                                    {#if showInstallPicker}
                                        <div class="loc-dropdown">
                                            {#if installDirOptions.length > 0}
                                                <div class="loc-header">Install to</div>
                                                {#each installDirOptions as opt}
                                                    <button class="loc-option" on:click|stopPropagation={() => { showInstallPicker = false; onChangeInstallDir && onChangeInstallDir(opt.path); }}>
                                                        <span class="loc-path">{opt.path}</span>
                                                        <span class="loc-label">{opt.label}</span>
                                                    </button>
                                                {/each}
                                                <div class="loc-divider"></div>
                                            {/if}
                                            <button class="loc-option" on:click|stopPropagation={() => { showInstallPicker = false; onChangeInstallDir && onChangeInstallDir(null); }}>
                                                <span class="loc-path" style="display: flex; align-items: center; gap: 5px;">
                                                    <FolderOpen size={11} /> Browse for custom folder…
                                                </span>
                                            </button>
                                        </div>
                                    {/if}
                                </div>
                            {/if}
                        </div>
                        <p class="text-[12px] mt-2" style="color: var(--label-quaternary);">
                            Where the game will be installed after extraction. Follows the archive location by default — override here if needed.
                        </p>
                    </div>
                {/if}
            </div>

        {/if}
        </div><!-- /scrollable body -->

        <!-- Actions — pinned to bottom -->
        {#if !loading && preflight}
            <div class="px-7 py-5 flex gap-3 flex-shrink-0" style="border-top: 1px solid var(--border-subtle);">
                <button on:click={onCancel} class="btn-secondary flex-1" style="justify-content: center; padding: 10px 16px; font-size: 13px;">
                    Cancel
                </button>
                <button
                    on:click={onConfirm}
                    disabled={!canInstall || notEnoughToDl}
                    class="flex-1 flex items-center justify-center gap-2 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    style="padding: 10px 16px; border-radius: 8px; font-size: 13px; font-weight: 700;
                           background: #f5f5f7; color: #0a0a0b; border: none;"
                    on:mouseenter={e => { if (canInstall && !notEnoughToDl) (e.currentTarget as HTMLElement).style.background = '#e5e5e7'; }}
                    on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = '#f5f5f7'; }}
                >
                    {#if sizeProbing}
                        <span class="inline-block w-3.5 h-3.5 rounded-full animate-spin" style="border: 1.5px solid rgba(10,10,11,0.2); border-top-color: #0a0a0b;"></span>
                        Checking…
                    {:else}
                        <Download size={15} />
                        Download
                    {/if}
                </button>
            </div>
        {/if}
    </div>
</div>

<svelte:window on:click={() => { showDownloadPicker = false; showInstallPicker = false; }} />

<style>
    .loc-dropdown {
        position: absolute;
        top: calc(100% + 5px);
        right: 0;
        z-index: 300;
        background: #1c1c1e;
        border: 1px solid rgba(255,255,255,0.12);
        border-radius: 9px;
        padding: 5px;
        min-width: 280px;
        box-shadow: 0 8px 28px rgba(0,0,0,0.6);
    }

    .loc-header {
        font-size: 9px;
        font-weight: 700;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: rgba(255,255,255,0.3);
        padding: 3px 8px 5px;
    }

    .loc-option {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        width: 100%;
        text-align: left;
        padding: 7px 9px;
        border-radius: 6px;
        border: none;
        background: transparent;
        cursor: pointer;
        transition: background 0.1s;
        gap: 1px;
    }
    .loc-option:hover { background: rgba(255,255,255,0.07); }

    .loc-path {
        font-family: ui-monospace, 'Cascadia Code', monospace;
        font-size: 10px;
        font-weight: 600;
        color: rgba(255,255,255,0.85);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        max-width: 190px;
    }

    .loc-label {
        font-size: 9px;
        color: rgba(255,255,255,0.35);
    }

    .loc-divider {
        height: 1px;
        background: rgba(255,255,255,0.07);
        margin: 4px 0;
    }
</style>

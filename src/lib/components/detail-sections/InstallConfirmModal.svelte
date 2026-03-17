<script lang="ts">
    import { HardDrive, FolderOpen, Download, AlertTriangle, XCircle, CheckCircle, Loader2, X } from 'lucide-svelte';

    export let gameTitle: string = '';
    export let coverUrl: string = '';

    export let preflight: {
        download_size_bytes: number;
        install_path: string;
        available_bytes: number;
    } | null = null;

    export let loading: boolean = false;
    export let onConfirm: () => void;
    export let onCancel: () => void;

    const COMPRESSION_RATIO = 2.5;

    function formatBytes(bytes: number): string {
        if (bytes <= 0) return 'Unknown';
        if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(1) + ' GB';
        if (bytes >= 1_048_576)     return (bytes / 1_048_576).toFixed(1) + ' MB';
        return (bytes / 1024).toFixed(0) + ' KB';
    }

    function shortenPath(p: string): string {
        const sep = p.includes('\\') ? '\\' : '/';
        const parts = p.split(sep).filter(Boolean);
        if (parts.length <= 4) return p;
        return parts[0] + sep + '...' + sep + parts.slice(-2).join(sep);
    }

    $: downloadSize     = preflight?.download_size_bytes ?? 0;
    $: available        = preflight?.available_bytes ?? 0;
    $: installPath      = preflight?.install_path ?? '';
    $: estimatedInstall = downloadSize > 0 ? Math.round(downloadSize * COMPRESSION_RATIO) : 0;
    $: totalRequired    = downloadSize > 0 ? downloadSize + estimatedInstall : 0;
    $: sizeKnown        = downloadSize > 0 && available > 0;
    $: notEnoughToDl    = sizeKnown && available < downloadSize;
    $: notEnoughToInstall = sizeKnown && !notEnoughToDl && available < totalRequired;
    $: canInstall       = !loading && !notEnoughToDl;
    $: drive            = installPath ? installPath.split(/[\\/]/)[0] : '';
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
        class="relative w-full max-w-md mx-4 overflow-hidden"
        style="background: var(--bg-surface); border: 1px solid var(--border); border-radius: 12px; box-shadow: var(--shadow-dialog);"
        on:click|stopPropagation
        on:keydown|stopPropagation
    >
        <!-- Header -->
        <div class="flex items-start justify-between px-5 pt-5 pb-4" style="border-bottom: 1px solid var(--border-subtle);">
            <div class="flex items-center gap-2.5">
                <div class="w-8 h-8 rounded-[6px] flex items-center justify-center"
                     style="border: 1px solid var(--border); background: rgba(255,255,255,0.04);">
                    <Download size={14} style="color: var(--label-tertiary);" />
                </div>
                <div>
                    <h2 class="text-[12px] font-semibold" style="color: var(--label-primary); letter-spacing: -0.01em;">Install Game</h2>
                    <p class="text-[11px] mt-0.5 max-w-[280px] truncate" style="color: var(--label-tertiary);">{gameTitle}</p>
                </div>
            </div>
            <button
                on:click={onCancel}
                class="p-1.5 rounded-[5px] transition-colors"
                style="color: var(--label-tertiary);"
                on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.07)'; (e.currentTarget as HTMLElement).style.color = 'var(--label-primary)'; }}
                on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'transparent'; (e.currentTarget as HTMLElement).style.color = 'var(--label-tertiary)'; }}
            >
                <X size={14} />
            </button>
        </div>

        {#if loading}
            <div class="px-5 py-8 flex flex-col items-center gap-3">
                <div class="w-6 h-6 rounded-full animate-spin"
                     style="border: 1.5px solid rgba(255,255,255,0.1); border-top-color: rgba(255,255,255,0.5);"></div>
                <p class="text-[12px]" style="color: var(--label-tertiary);">Checking disk space…</p>
            </div>

        {:else if preflight}
            <!-- Install location -->
            <div class="px-5 pt-4 pb-3">
                <p class="text-[11px] font-medium uppercase mb-2" style="letter-spacing: 0.06em; color: var(--label-tertiary);">Install Location</p>
                <div class="flex items-center gap-2.5 px-3 py-2.5 rounded-[6px]"
                     style="background: var(--bg-raised); border: 1px solid var(--border-subtle);">
                    <FolderOpen size={13} style="color: var(--label-tertiary); flex-shrink: 0;" />
                    <p class="text-[11px] font-mono truncate" style="color: var(--label-secondary);" title={installPath}>
                        {shortenPath(installPath)}
                    </p>
                </div>
            </div>

            <!-- Disk space table -->
            <div class="px-5 pb-3">
                <p class="text-[11px] font-medium uppercase mb-2" style="letter-spacing: 0.06em; color: var(--label-tertiary);">Disk Space</p>
                <div class="rounded-[6px] overflow-hidden" style="border: 1px solid var(--border-subtle);">
                    <table class="w-full">
                        <tbody>
                            <tr style="border-bottom: 1px solid var(--border-subtle);">
                                <td class="px-3 py-2.5 text-[11px] flex items-center gap-1.5" style="color: var(--label-tertiary);">
                                    <Download size={11} class="shrink-0" />
                                    Download size
                                </td>
                                <td class="px-3 py-2.5 text-right text-[11px] font-medium" style="color: var(--label-secondary);">
                                    {downloadSize > 0 ? formatBytes(downloadSize) : 'Unknown'}
                                </td>
                            </tr>

                            {#if estimatedInstall > 0}
                                <tr style="border-bottom: 1px solid var(--border-subtle);">
                                    <td class="px-3 py-2.5 text-[11px] flex items-center gap-1.5" style="color: var(--label-tertiary);">
                                        <HardDrive size={11} class="shrink-0" />
                                        Estimated installed
                                    </td>
                                    <td class="px-3 py-2.5 text-right text-[11px]" style="color: var(--label-tertiary);">
                                        ~{formatBytes(estimatedInstall)}
                                    </td>
                                </tr>

                                <tr style="border-bottom: 1px solid var(--border-subtle); background: rgba(255,255,255,0.02);">
                                    <td class="px-3 py-2.5 text-[11px] font-medium" style="color: var(--label-secondary);">Total required</td>
                                    <td class="px-3 py-2.5 text-right text-[11px] font-bold" style="color: var(--label-primary);">
                                        ~{formatBytes(totalRequired)}
                                    </td>
                                </tr>
                            {/if}

                            <tr>
                                <td class="px-3 py-2.5 text-[11px] font-medium flex items-center gap-1.5"
                                    style="color: {notEnoughToDl ? '#ff453a' : notEnoughToInstall ? '#ff9f0a' : 'var(--label-secondary)'};">
                                    {#if notEnoughToDl}
                                        <XCircle size={11} class="shrink-0" />
                                    {:else if notEnoughToInstall}
                                        <AlertTriangle size={11} class="shrink-0" />
                                    {:else}
                                        <CheckCircle size={11} class="shrink-0" style="color: #32d74b;" />
                                    {/if}
                                    Available ({drive})
                                </td>
                                <td class="px-3 py-2.5 text-right text-[11px] font-bold"
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
                <div class="mx-5 mb-3 flex items-start gap-2.5 px-3 py-2.5 rounded-[6px]"
                     style="border: 1px solid rgba(255,69,58,0.25);">
                    <XCircle size={13} style="color: #ff453a; flex-shrink: 0; margin-top: 1px;" />
                    <p class="text-[11px] leading-relaxed" style="color: rgba(255,69,58,0.85);">
                        <span style="font-weight: 700; color: #ff453a;">Not enough space.</span>
                        {#if totalRequired > 0}
                            Requires ~{formatBytes(totalRequired)}, only {formatBytes(available)} available on {drive}.
                        {:else}
                            Free up space on {drive} and try again.
                        {/if}
                    </p>
                </div>
            {:else if notEnoughToInstall}
                <div class="mx-5 mb-3 flex items-start gap-2.5 px-3 py-2.5 rounded-[6px]"
                     style="border: 1px solid rgba(255,159,10,0.25);">
                    <AlertTriangle size={13} style="color: #ff9f0a; flex-shrink: 0; margin-top: 1px;" />
                    <p class="text-[11px] leading-relaxed" style="color: rgba(255,159,10,0.85);">
                        You may not have enough space to extract. Consider freeing up space on {drive}.
                    </p>
                </div>
            {:else if !sizeKnown}
                <div class="mx-5 mb-3 flex items-start gap-2.5 px-3 py-2.5 rounded-[6px]"
                     style="border: 1px solid var(--border-subtle);">
                    <AlertTriangle size={13} style="color: var(--label-tertiary); flex-shrink: 0; margin-top: 1px;" />
                    <p class="text-[11px]" style="color: var(--label-tertiary);">
                        Could not determine file size. Ensure you have enough free space.
                    </p>
                </div>
            {/if}

            <!-- Actions -->
            <div class="px-5 pb-5 flex gap-2">
                <button on:click={onCancel} class="btn-secondary flex-1" style="justify-content: center;">
                    Cancel
                </button>
                <button
                    on:click={onConfirm}
                    disabled={!canInstall}
                    class="flex-1 flex items-center justify-center gap-1.5 transition-colors disabled:cursor-not-allowed"
                    style="padding: 8px 16px; border-radius: 8px; font-size: 12px; font-weight: 700;
                           {notEnoughToDl
                               ? 'border: 1px solid rgba(255,69,58,0.3); color: #ff453a; background: transparent; opacity: 0.6;'
                               : 'background: #f5f5f7; color: #0a0a0b; border: none;'}"
                    on:mouseenter={e => { if (!notEnoughToDl && canInstall) (e.currentTarget as HTMLElement).style.background = '#e5e5e7'; }}
                    on:mouseleave={e => { if (!notEnoughToDl) (e.currentTarget as HTMLElement).style.background = '#f5f5f7'; }}
                >
                    {#if notEnoughToDl}
                        <XCircle size={13} />
                        Not enough space
                    {:else}
                        <Download size={13} />
                        Install
                    {/if}
                </button>
            </div>
        {/if}
    </div>
</div>

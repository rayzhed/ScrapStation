<script lang="ts">
    import * as LucideIcons from 'lucide-svelte';
    import { ExternalLink, Download, Loader2, AlertCircle, Globe, Lock, AlertTriangle } from 'lucide-svelte';
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { open as openDialog } from '@tauri-apps/plugin-dialog';
    import { storageConfig, getKnownLibraryLocations, type KnownLibraryLocation } from '$lib/stores/appSettings';
    import type { DetectedHost, SmartDownloadResult, DownloadButton } from '$lib/types';
    import InstallConfirmModal from './InstallConfirmModal.svelte';
    import NoticeModal from '../NoticeModal.svelte';
    import {
        addDownload,
        enqueueDownload,
        failDownload,
        updateDownloadProgress
    } from '$lib/stores/downloads';
    import {
        addGameToLibrary,
        linkDownloadToLibrary,
        isGameInLibrary,
        updateArchivePassword,
    } from '$lib/stores/library';

    export let title: string;
    export let icon: string;
    export let data: { buttons: DownloadButton[] };
    export let sourceId: string;
    export let gameTitle: string = 'Unknown Game';
    export let gameUrl: string = '';
    export let coverUrl: string = '';
    export let archivePassword: string | undefined = undefined;

    // Track library game ID for linking downloads
    let libraryGameId: string | null = null;

    // Ensure game is in library before downloading
    async function ensureGameInLibrary(): Promise<string | null> {
        if (libraryGameId) return libraryGameId;
        if (!gameUrl) {
            return null;
        }

        try {
            // Check if already in library
            const existing = await isGameInLibrary(sourceId, gameUrl);
            if (existing) {
                libraryGameId = existing.id;

                // Patch missing archive password
                if (archivePassword && !existing.archive_password) {
                    await updateArchivePassword(existing.id, archivePassword);
                }

                // Patch missing cover URL — triggers background download automatically
                if (coverUrl && !existing.cover_url) {
                    invoke('update_game_cover_url', {
                        gameId: existing.id,
                        coverUrl,
                    }).catch(() => {});
                }

                return libraryGameId;
            }

            // Add to library with cover + password
            libraryGameId = await addGameToLibrary({
                sourceSlug: sourceId,
                sourceGameId: gameUrl,
                title: gameTitle,
                coverUrl: coverUrl || undefined,
                archivePassword: archivePassword,
            });
            return libraryGameId;
        } catch (e) {
            return null;
        }
    }

    interface ResolvedLink {
        url: string;
        label: string | null;
        host: string | null;
        size: string | null;
        browser_only: boolean;
        browser_only_reason: string | null;
        metadata: Record<string, string>;
    }

    interface ButtonState {
        loading: boolean;
        status: string;
        error: string | null;
        fallbackUrl: string | null;
        detectedHost: DetectedHost | null;
        resolvedLinks: ResolvedLink[] | null;
    }

    let visible = false;
    let buttonStates: { [key: number]: ButtonState } = {};
    let resolvedLinkHosts: Record<string, DetectedHost | null> = {};

    // ── Per-download folder selection ────────────────────────────────────────
    // null = use the configured default (storageConfig.effective_download_path)
    let chosenDownloadDir: string | null = null;
    // null = follow download dir (auto-derived), or an explicit override
    let chosenInstallDir: string | null = null;
    $: effectiveDownloadDir = chosenDownloadDir ?? $storageConfig?.effective_download_path ?? null;

    /**
     * Effective install dir: explicit override → or derived from chosen download dir
     * (replaces \ScrapStation\Downloads with \ScrapStation\Library) → or null (backend default).
     */
    function getEffectiveInstallDir(): string | null {
        if (chosenInstallDir !== null) return chosenInstallDir;
        if (chosenDownloadDir !== null) {
            const sep = chosenDownloadDir.includes('\\') ? '\\' : '/';
            return chosenDownloadDir.replace(/[/\\]ScrapStation[/\\]Downloads[/\\]?$/i, '') + sep + 'ScrapStation' + sep + 'Library';
        }
        return null;
    }

    // Known library locations (loaded once, used to build picker options)
    let knownLocations: KnownLibraryLocation[] = [];

    /** Options for the download dir picker: derive {root}\ScrapStation\Downloads from each known location */
    $: downloadDirOptions = knownLocations.map(l => ({
        path: l.path.replace(/[/\\]ScrapStation[/\\]Library[/\\]?$/i, '') + '\\ScrapStation\\Downloads',
        label: l.label,
    })).filter((opt, i, arr) => {
        const key = opt.path.toLowerCase();
        return arr.findIndex(o => o.path.toLowerCase() === key) === i;
    });

    /** Options for the install dir picker: directly the known library locations */
    $: installDirOptions = knownLocations.map(l => ({ path: l.path, label: l.label }));

    // ── Per-button warning confirmation ─────────────────────────────────────
    let pendingWarning: { message: string; action: () => void } | null = null;

    function withButtonWarning(button: DownloadButton, action: () => void) {
        if (button.warning) {
            pendingWarning = { message: button.warning, action };
        } else {
            action();
        }
    }

    // ── Source notices (download_start modals) ───────────────────────────────
    let sourceNotices: any[] = [];
    let pendingNotice: any | null = null;
    let _pendingNoticeAction: (() => void) | null = null;

    function getUnseenDownloadNotices(): any[] {
        return sourceNotices.filter((n: any) => {
            if (n.trigger !== 'download_start') return false;
            if (n.once && localStorage.getItem(`notice_seen_${sourceId}_${n.id}`)) return false;
            return true;
        });
    }

    function withDownloadNotice(action: () => void) {
        const unseen = getUnseenDownloadNotices();
        if (unseen.length > 0) {
            pendingNotice = unseen[0];
            _pendingNoticeAction = action;
            return;
        }
        action();
    }

    function onNoticeConfirm() {
        const action = _pendingNoticeAction;
        pendingNotice = null;
        _pendingNoticeAction = null;
        if (action) action();
    }

    function onNoticeCancel() {
        pendingNotice = null;
        _pendingNoticeAction = null;
    }

    // ── Install preflight (disk-space check modal) ───────────────────────────
    type PendingInstall =
        | { type: 'resolved'; link: ResolvedLink; buttonIndex: number; linkIndex: number }
        | { type: 'smart'; button: DownloadButton; buttonIndex: number };
    let pendingInstall: PendingInstall | null = null;
    let preflightInfo: { download_size_bytes: number; download_path: string; install_path: string; available_bytes: number } | null = null;
    let preflightLoading = false;
    // True while probe_download_size is running (size still unknown)
    let sizeProbing = false;
    // Probe result: resolved direct URL + optional webview cookies — reused when confirming install
    let probedUrl: string | null = null;
    let probedCookies: string | null = null;

    // Initialize button states
    $: if (data?.buttons) {
        data.buttons.forEach((_: any, index: number) => {
            if (!buttonStates[index]) {
                buttonStates[index] = {
                    loading: false,
                    status: '',
                    error: null,
                    fallbackUrl: null,
                    detectedHost: null,
                    resolvedLinks: null
                };
            }
        });
    }

    onMount(async () => {
        setTimeout(() => visible = true, 200);
        // Fetch source notices for download_start modals
        try {
            const meta = await invoke<any>('get_source_metadata', { sourceId });
            sourceNotices = meta.notices || [];
        } catch {}
        // Detect hosts for all buttons
        detectHosts();
        // Auto-resolve buttons that have a resolver path (e.g., hosters)
        autoResolveButtons();
        // Load known library locations for the picker dropdowns
        try { knownLocations = await getKnownLibraryLocations(); } catch {}
    });

    // Automatically resolve buttons that have a resolver (e.g., hosters_page)
    async function autoResolveButtons() {
        if (!data?.buttons) return;

        // Resolve all buttons with resolvers in parallel for faster loading
        const resolvePromises = data.buttons.map(async (button, i) => {
            if ((button as any).resolver) {
                await navigateWithResolver(button, i);
            }
        });

        await Promise.all(resolvePromises);
    }

    async function detectHosts() {
        if (!data?.buttons) return;

        for (let i = 0; i < data.buttons.length; i++) {
            const button = data.buttons[i];
            if (button.url) {
                try {
                    // Pass resolve_first: true if button has resolve_link enabled
                    const host = await invoke<DetectedHost>('detect_host', {
                        url: button.url,
                        sourceId: sourceId,
                        resolveFirst: button.resolve_link || false
                    });
                    buttonStates[i] = { ...buttonStates[i], detectedHost: host };
                    buttonStates = buttonStates; // Trigger reactivity
                } catch (e) {
                }
            }
        }
    }

    function getIconComponent(iconName: string) {
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        return (LucideIcons as unknown as Record<string, typeof LucideIcons.Download>)[pascalCase] || LucideIcons.Download;
    }

    function getButtonLabel(button: DownloadButton, index: number): string {
        // Priority: detected host label (if smart_download) > button.label > 'Download'
        const host = buttonStates[index]?.detectedHost;

        // If smart_download is enabled and we detected a known host, use its label
        if (button.smart_download && host?.label && !host.host_id.startsWith('auto:') && host.host_id !== 'unknown') {
            return host.label;
        }

        // Otherwise use button label or fallback
        return button.label || host?.label || 'Download';
    }

    function getButtonIcon(button: DownloadButton, index: number): string {
        // Use detected host icon if available
        const host = buttonStates[index]?.detectedHost;
        if (host?.icon) {
            return host.icon;
        }
        return button.icon || 'download';
    }

    function getButtonColor(button: DownloadButton, index: number): string | null {
        const host = buttonStates[index]?.detectedHost;
        return host?.color || null;
    }

    async function handleButtonClick(button: DownloadButton, index: number) {
        withDownloadNotice(() => withButtonWarning(button, () => _executeButtonAction(button, index)));
    }

    async function _executeButtonAction(button: DownloadButton, index: number) {
        // Determine action: smart_download flag takes precedence, then action field
        let action = button.action || 'open_link';
        if (button.smart_download) {
            action = 'smart_download';
        }

        // Clear previous error
        buttonStates[index] = {
            ...buttonStates[index],
            error: null,
            fallbackUrl: null
        };

        if (action === 'smart_download') {
            await smartDownload(button, index);
        } else if (action === 'download_file') {
            await downloadFile(button, index);
        } else if (action === 'open_link') {
            await openLink(button, index);
        }
    }

    // Open the preflight modal: instantly show install path + disk space,
    // then probe the real archive size in the background and update the modal.
    async function _openPreflightModal(
        install: PendingInstall,
        probeUrl: string,
    ) {
        pendingInstall = install;
        preflightInfo = null;
        preflightLoading = true;
        sizeProbing = false;
        probedUrl = null;
        probedCookies = null;

        // Fast call: install path + available bytes only (no network probe)
        // Read chosenDownloadDir directly (not effectiveDownloadDir, which may be stale due to Svelte batch updates)
        const currentDownloadDir = chosenDownloadDir ?? $storageConfig?.effective_download_path ?? null;
        const currentInstallDir = getEffectiveInstallDir();
        try {
            const pf = await invoke<{ download_size_bytes: number; download_path: string; install_path: string; available_bytes: number }>(
                'get_install_preflight',
                { url: probeUrl, sourceId, gameTitle, downloadDir: currentDownloadDir, installDir: currentInstallDir }
            );
            preflightInfo = { download_size_bytes: 0, download_path: pf.download_path, install_path: pf.install_path, available_bytes: pf.available_bytes };
        } catch {
            preflightInfo = { download_size_bytes: 0, download_path: currentDownloadDir ?? '', install_path: '', available_bytes: 0 };
        } finally {
            preflightLoading = false;
        }

        // Probe the real size via the full download pipeline (no file written).
        // Also captures the resolved URL + cookies so the actual download can skip re-resolution.
        sizeProbing = true;
        invoke<{ size: number; resolved_url: string; cookies: string | null }>('probe_download_size', { url: probeUrl, sourceId })
            .then(result => {
                if (pendingInstall && preflightInfo && result.size > 0) {
                    preflightInfo = { ...preflightInfo, download_size_bytes: result.size };
                }
                // Store resolved URL + cookies for instant download start on confirm
                probedUrl = result.resolved_url || null;
                probedCookies = result.cookies ?? null;
            })
            .catch(() => {})
            .finally(() => { sizeProbing = false; });
    }

    // Change the download folder from within the preflight modal
    // path = selected location, null = open system folder picker
    async function handleChangeDownloadDir(path: string | null) {
        let chosen: string | null = path;
        if (chosen === null) {
            const result = await openDialog({ directory: true, title: 'Choose Download Folder' });
            if (!result) return;
            chosen = result as string;
        }
        chosenDownloadDir = chosen;
        if (pendingInstall) {
            const probeUrl = pendingInstall.type === 'smart' ? pendingInstall.button.url : pendingInstall.link.url;
            await _openPreflightModal(pendingInstall, probeUrl);
        }
    }

    // Change the install folder from within the preflight modal (Advanced)
    // path = selected location, null = open system folder picker
    async function handleChangeInstallDir(path: string | null) {
        let chosen: string | null = path;
        if (chosen === null) {
            const result = await openDialog({ directory: true, title: 'Choose Install Folder' });
            if (!result) return;
            chosen = result as string;
        }
        chosenInstallDir = chosen;
        if (pendingInstall) {
            const probeUrl = pendingInstall.type === 'smart' ? pendingInstall.button.url : pendingInstall.link.url;
            await _openPreflightModal(pendingInstall, probeUrl);
        }
    }

    // Show preflight modal, then start download on confirm
    async function smartDownload(button: DownloadButton, index: number) {
        buttonStates[index] = { ...buttonStates[index], loading: true, status: 'Checking…' };
        buttonStates = buttonStates;

        await _openPreflightModal({ type: 'smart', button, buttonIndex: index }, button.url);

        buttonStates[index] = { ...buttonStates[index], loading: false, status: '' };
        buttonStates = buttonStates;
    }

    // Actual download logic after preflight confirmation
    async function _startSmartDownload(button: DownloadButton, index: number) {
        buttonStates[index] = { ...buttonStates[index], loading: true, status: 'Resolving download link...' };
        buttonStates = buttonStates;

        const gameId = await ensureGameInLibrary();

        const urlFilename = button.url.split('/').pop()?.split('?')[0] || 'download';
        const fileName = urlFilename.includes('.') ? urlFilename : `${urlFilename}.rar`;
        const hostLabel = buttonStates[index]?.detectedHost?.label || 'Unknown';
        const hostColor = buttonStates[index]?.detectedHost?.color || '#00e5ff';

        const downloadId = addDownload({
            gameTitle,
            fileName,
            url: button.url,
            sourceId,
            hostLabel,
            hostColor
        });

        if (gameId) {
            await linkDownloadToLibrary(gameId, downloadId);
        }

        try {
            await enqueueDownload({
                downloadId,
                url: button.url,
                sourceId,
                filenameHint: (button as any).filename || null,
                preResolvedUrl: probedUrl,
                preCookies: probedCookies,
                downloadDir: chosenDownloadDir,
                installDir: getEffectiveInstallDir(),
            });

            buttonStates[index] = { ...buttonStates[index], status: 'Downloading…', loading: false };
            setTimeout(() => {
                buttonStates[index] = { ...buttonStates[index], status: '' };
                buttonStates = buttonStates;
            }, 3000);
        } catch (error: any) {
            failDownload(downloadId, error.toString());
            buttonStates[index] = {
                ...buttonStates[index],
                loading: false,
                status: '',
                error: error.toString(),
                fallbackUrl: button.url
            };
        }

        buttonStates = buttonStates;
    }

    async function openLink(button: DownloadButton, index: number) {
        // Check if button has a resolver (path-based navigation)
        if ((button as any).resolver) {
            await navigateWithResolver(button, index);
            return;
        }

        if (button.resolve_link) {
            buttonStates[index] = { ...buttonStates[index], loading: true, status: 'Resolving link...' };
            buttonStates = buttonStates;

            try {
                const resolvedUrl = await resolveLink(button.url);
                buttonStates[index] = { ...buttonStates[index], status: 'Opening...' };
                await invoke('open_url_in_browser', { url: resolvedUrl });
                buttonStates[index] = { ...buttonStates[index], status: 'Opened!' };

                setTimeout(() => {
                    buttonStates[index] = { ...buttonStates[index], status: '', loading: false };
                    buttonStates = buttonStates;
                }, 1000);
            } catch (error: any) {
                buttonStates[index] = {
                    ...buttonStates[index],
                    loading: false,
                    status: '',
                    error: error.toString(),
                    fallbackUrl: button.url
                };
                buttonStates = buttonStates;
            }
        } else {
            try {
                await invoke('open_url_in_browser', { url: button.url });
            } catch (error) {
            }
        }
    }

    // New function for path-based navigation (hosters, etc.)
    async function navigateWithResolver(button: DownloadButton, index: number) {
        buttonStates[index] = { ...buttonStates[index], loading: true, status: 'Resolving download links...' };
        buttonStates = buttonStates;

        try {
            const result = await invoke<{ links: ResolvedLink[], warnings: string[], duration_ms: number }>('navigate_link', {
                url: button.url,
                sourceId: sourceId,
                pathName: (button as any).resolver
            });

            // Handle case where result might not have expected structure
            const links = result?.links || [];

            if (links.length === 0) {
                buttonStates[index] = {
                    ...buttonStates[index],
                    loading: false,
                    status: '',
                    error: 'No download links found',
                    fallbackUrl: button.url,
                    resolvedLinks: null
                };
            } else {
                // Store resolved links - they will be displayed as separate buttons
                buttonStates[index] = {
                    ...buttonStates[index],
                    loading: false,
                    status: '',
                    resolvedLinks: links
                };

                // Detect hosts for all resolved links in parallel to get proper labels/icons
                const hostDetections = await Promise.all(
                    links.map(async (link: ResolvedLink) => {
                        try {
                            const host = await invoke<DetectedHost>('detect_host', {
                                url: link.url,
                                sourceId,
                                resolveFirst: false
                            });
                            return [link.url, host] as [string, DetectedHost];
                        } catch {
                            return [link.url, null] as [string, null];
                        }
                    })
                );
                for (const [url, host] of hostDetections) {
                    resolvedLinkHosts[url] = host;
                }
                resolvedLinkHosts = resolvedLinkHosts;
            }

        } catch (error: any) {
            buttonStates[index] = {
                ...buttonStates[index],
                loading: false,
                status: '',
                error: error.toString(),
                fallbackUrl: button.url,
                resolvedLinks: null
            };
        }

        buttonStates = buttonStates;
    }

    // Show install-preflight modal before downloading a resolved link
    async function openResolvedLink(link: ResolvedLink, buttonIndex: number, linkIndex: number) {
        await _openPreflightModal({ type: 'resolved', link, buttonIndex, linkIndex }, link.url);
    }

    function cancelInstall() {
        pendingInstall = null;
        preflightInfo = null;
        preflightLoading = false;
        sizeProbing = false;
        probedUrl = null;
        probedCookies = null;
    }

    async function confirmInstall() {
        if (!pendingInstall) return;
        const inst = pendingInstall;
        pendingInstall = null;
        preflightInfo = null;
        if (inst.type === 'resolved') {
            await _startResolvedDownload(inst.link, inst.buttonIndex, inst.linkIndex);
        } else if (inst.type === 'smart') {
            await _startSmartDownload(inst.button, inst.buttonIndex);
        }
    }

    // Actual download logic (called after user confirms in the modal)
    async function _startResolvedDownload(link: ResolvedLink, buttonIndex: number, linkIndex: number) {

        // Ensure game is in library first
        const gameId = await ensureGameInLibrary();

        // Extract filename from URL or use label
        const urlFilename = link.url.split('/').pop()?.split('?')[0] || 'download';
        const fileName = urlFilename.includes('.') ? urlFilename : `${urlFilename}.rar`;
        const hostLabel = link.host || link.label || getResolvedLinkLabel(link);

        // Add to downloads store
        const downloadId = addDownload({
            gameTitle,
            fileName,
            url: link.url,
            sourceId,
            hostLabel,
            hostColor: '#00e5ff'
        });

        // Link download to library game
        if (gameId) {
            await linkDownloadToLibrary(gameId, downloadId);
        }

        try {
            await enqueueDownload({
                downloadId,
                url: link.url,
                sourceId,
                filenameHint: null,
                preResolvedUrl: probedUrl,
                preCookies: probedCookies,
                downloadDir: chosenDownloadDir,
                installDir: getEffectiveInstallDir(),
            });
            // Progress and completion will come via events
        } catch (error: any) {
            failDownload(downloadId, error.toString());
            await invoke('open_url_in_browser', { url: link.url });
        }
    }

    // Get a label for a resolved link
    function getResolvedLinkLabel(link: ResolvedLink): string {
        // 1. Explicit label from YAML extract_meta
        if (link.label) return link.label;
        if (link.host) return link.host;

        // 2. Detected host label (e.g. "GoFile", "PixelDrain") from host config
        const detected = resolvedLinkHosts[link.url];
        if (detected?.label && detected.host_id !== 'unknown' && !detected.host_id.startsWith('auto:')) {
            return detected.label;
        }

        // 3. Extract domain from URL, normalizing JSON-escaped slashes (\/ → /)
        try {
            const normalized = link.url.replace(/\\\//g, '/');
            const url = new URL(normalized);
            return url.hostname.replace('www.', '');
        } catch {
            return 'Download';
        }
    }

    function getResolvedLinkIcon(link: ResolvedLink): string {
        const detected = resolvedLinkHosts[link.url];
        if (detected?.icon) return detected.icon;
        return 'cloud-download';
    }

    function getResolvedLinkColor(link: ResolvedLink): string | null {
        const detected = resolvedLinkHosts[link.url];
        return detected?.color || null;
    }

    // ---- Link classification helpers ----

    const MULTI_PART_PATTERNS_LOCAL = [
        /\.part\d+\.rar$/i,
        /\.r\d{2}$/i,
        /\.7z\.\d{3}$/i,
        /\.zip\.\d{3}$/i,
        /\.z\d{2}$/i,
    ];

    function isMultiPartLink(url: string): boolean {
        const filename = url.split('/').pop()?.split('?')[0] || '';
        return MULTI_PART_PATTERNS_LOCAL.some(p => p.test(filename));
    }

    function classifyLink(link: ResolvedLink): 'game' | 'fix' {
        // 1. Explicit type set by the source's path resolver — highest priority, fully generic.
        const explicit = link.metadata?.link_type as string | undefined;
        if (explicit === 'fix') return 'fix';
        if (explicit === 'game') return 'game';

        // 2. Keyword fallback for sources that don't provide link_type.
        //    Use letter-only boundaries so _Fix_ / _Repair_ (underscore-separated) are caught.
        const metaFilename = (link.metadata?.file_name || '').toLowerCase();
        const urlFilename = (link.url.split('/').pop()?.split('?')[0] || '').toLowerCase();
        const filename = metaFilename || urlFilename;
        const label = (link.label || '').toLowerCase();
        const text = `${filename} ${label}`;

        if (/(?<![a-zA-Z])(?:fix|repair|patch|hotfix|crack)(?![a-zA-Z])/i.test(text)) return 'fix';

        return 'game';
    }

    // True only when game links have 2+ DISTINCT filenames (real multipart archive).
    // 5 mirrors of the same file → same filename → not multipart.
    function hasDistinctGameParts(links: ResolvedLink[]): boolean {
        const gameLinks = links.filter(l => classifyLink(l) === 'game');
        const filenames = new Set(
            gameLinks.map(l =>
                (l.metadata?.file_name || l.url.split('/').pop()?.split('?')[0] || '').toLowerCase()
            )
        );
        return filenames.size > 1;
    }

    function sortedResolvedLinks(links: ResolvedLink[]): ResolvedLink[] {
        // Game archives first, fix archives last
        return [...links].sort((a, b) => {
            const aType = classifyLink(a) === 'fix' ? 1 : 0;
            const bType = classifyLink(b) === 'fix' ? 1 : 0;
            return aType - bType;
        });
    }

    // Download only game-classified links from a resolved set
    // Uses _startResolvedDownload directly (no per-link modal for batch operations)
    async function downloadAllGameArchives(links: ResolvedLink[], buttonIndex: number) {
        const gameLinks = links.filter(l => classifyLink(l) === 'game');
        for (const link of gameLinks) {
            await _startResolvedDownload(link, buttonIndex, links.indexOf(link));
        }
    }

    async function resolveLink(url: string): Promise<string> {
        const response = await invoke('resolve_download_link', {
            url: url,
            sourceId: sourceId,
        });
        return response as string;
    }

    async function downloadFile(button: DownloadButton, index: number) {
        buttonStates[index] = { ...buttonStates[index], loading: true, status: 'Downloading...' };
        buttonStates = buttonStates;

        try {
            const filePath = await invoke<string>('download_file', {
                url: button.url,
                sourceId: sourceId,
                resolveLink: button.resolve_link || false,
            });

            buttonStates[index] = { ...buttonStates[index], status: 'Downloaded!', loading: false };

            setTimeout(() => {
                buttonStates[index] = { ...buttonStates[index], status: '' };
                buttonStates = buttonStates;
            }, 3000);
        } catch (error: any) {
            buttonStates[index] = {
                ...buttonStates[index],
                loading: false,
                status: '',
                error: error.toString(),
                fallbackUrl: button.url
            };
            buttonStates = buttonStates;
        }
    }

    async function openFallback(index: number) {
        const fallbackUrl = buttonStates[index]?.fallbackUrl;
        if (fallbackUrl) {
            try {
                await invoke('open_url_in_browser', { url: fallbackUrl });
                // Clear error after opening
                buttonStates[index] = { ...buttonStates[index], error: null, fallbackUrl: null };
                buttonStates = buttonStates;
            } catch (e) {
            }
        }
    }

    function dismissError(index: number) {
        buttonStates[index] = { ...buttonStates[index], error: null, fallbackUrl: null };
        buttonStates = buttonStates;
    }

    $: IconComponent = getIconComponent(icon);
</script>

<!-- Glassy Download Section -->
<div class="mb-12 transition-all duration-700 {visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8'}">
    <!-- Section Header -->
    <div class="flex items-center gap-3 mb-6 px-1">
        <svelte:component this={IconComponent} size={20} strokeWidth={2} class="text-white/60" />
        <h2 class="text-2xl font-bold tracking-tight">{title}</h2>
    </div>

    <!-- Download Buttons - Enhanced Glass Grid -->
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {#each data.buttons as button, index}
            {@const buttonIcon = getButtonIcon(button, index)}
            {@const ButtonIconComponent = getIconComponent(buttonIcon)}
            {@const action = button.action || 'open_link'}
            {@const state = buttonStates[index] || { loading: false, status: '', error: null, fallbackUrl: null, detectedHost: null, resolvedLinks: null }}
            {@const buttonColor = getButtonColor(button, index)}
            {@const buttonLabel = getButtonLabel(button, index)}

            <!-- Check if this button has resolved links to display -->
            {#if state.resolvedLinks && state.resolvedLinks.length > 0}
                {@const sorted = sortedResolvedLinks(state.resolvedLinks)}
                {@const gameCount = sorted.filter(l => classifyLink(l) === 'game').length}
                <!-- "Download all parts" only for real multipart archives (distinct filenames per part) -->
                {#if hasDistinctGameParts(state.resolvedLinks!)}
                    <div class="col-span-full mb-1">
                        <button
                            on:click={() => downloadAllGameArchives(state.resolvedLinks!, index)}
                            class="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-[8px] transition-all text-[13px] font-medium"
                            style="background: rgba(255,255,255,0.05); border: 1px solid var(--border); color: var(--label-secondary);"
                            on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.09)'; (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-strong)'; }}
                            on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.05)'; (e.currentTarget as HTMLElement).style.borderColor = 'var(--border)'; }}
                        >
                            <svelte:component this={getIconComponent('download')} size={14} strokeWidth={2} />
                            Download all {gameCount} parts
                        </button>
                    </div>
                {/if}
                <!-- Render resolved links as separate buttons (game first, fix last) -->
                {#each sorted as resolvedLink, linkIndex}
                    {@const linkLabel = getResolvedLinkLabel(resolvedLink)}
                    {@const linkType = classifyLink(resolvedLink)}
                    {@const multiPart = isMultiPartLink(resolvedLink.url)}
                    {@const linkIcon = getResolvedLinkIcon(resolvedLink)}
                    {@const linkColor = getResolvedLinkColor(resolvedLink)}
                    <div class="relative">
                        <button
                            on:click={() => openResolvedLink(resolvedLink, index, linkIndex)}
                            class="w-full group flex items-center gap-4 p-5 rounded-[10px] transition-all duration-200 text-left"
                            style="background: {linkType === 'fix' ? 'rgba(255,159,10,0.06)' : 'var(--bg-surface)'}; border: 1px solid {linkType === 'fix' ? 'rgba(255,159,10,0.2)' : 'var(--border)'};"
                            on:mouseenter={e => { const el = e.currentTarget as HTMLElement; el.style.background = linkType === 'fix' ? 'rgba(255,159,10,0.1)' : 'var(--bg-raised)'; el.style.borderColor = linkType === 'fix' ? 'rgba(255,159,10,0.3)' : 'var(--border-strong)'; }}
                            on:mouseleave={e => { const el = e.currentTarget as HTMLElement; el.style.background = linkType === 'fix' ? 'rgba(255,159,10,0.06)' : 'var(--bg-surface)'; el.style.borderColor = linkType === 'fix' ? 'rgba(255,159,10,0.2)' : 'var(--border)'; }}
                        >
                            <!-- Icon -->
                            <div
                                class="flex-shrink-0 w-12 h-12 flex items-center justify-center rounded-[8px] transition-all"
                                style="border: 1px solid {linkType === 'fix' ? 'rgba(255,159,10,0.25)' : linkColor ? linkColor + '30' : 'var(--border)'}; background: {linkType === 'fix' ? 'rgba(255,159,10,0.1)' : 'rgba(255,255,255,0.05)'};"
                            >
                                <svelte:component
                                    this={getIconComponent(linkType === 'fix' ? 'wrench' : linkIcon)}
                                    size={20}
                                    strokeWidth={1.75}
                                    style="color: {linkType === 'fix' ? '#ff9f0a' : linkColor || 'var(--label-secondary)'};"
                                />
                            </div>

                            <!-- Content -->
                            <div class="flex-1 min-w-0">
                                <div class="flex items-center gap-2 min-w-0">
                                    <p class="text-[13px] font-semibold truncate" style="color: var(--label-primary);">{linkLabel}</p>
                                    {#if linkType === 'fix'}
                                        <span class="flex-shrink-0 text-[11px] px-1.5 py-0.5 rounded-[4px] font-medium"
                                              style="background: rgba(255,159,10,0.15); color: #ff9f0a; border: 1px solid rgba(255,159,10,0.25);">Fix</span>
                                    {:else if multiPart}
                                        <span class="flex-shrink-0 text-[11px] px-1.5 py-0.5 rounded-[4px] font-medium"
                                              style="background: rgba(10,132,255,0.15); color: #0a84ff; border: 1px solid rgba(10,132,255,0.25);">Part</span>
                                    {/if}
                                </div>
                                <p class="text-[12px] mt-0.5" style="color: {linkType === 'fix' ? 'rgba(255,159,10,0.6)' : 'var(--label-tertiary)'};">
                                    {#if resolvedLink.size}
                                        {resolvedLink.size}
                                    {:else if resolvedLink.browser_only}
                                        {resolvedLink.browser_only_reason || 'Opens in browser'}
                                    {:else if linkType === 'fix'}
                                        Patch / fix archive
                                    {:else}
                                        Game archive
                                    {/if}
                                </p>
                            </div>

                            <!-- Arrow -->
                            <ExternalLink size={16} strokeWidth={1.75} style="color: var(--label-tertiary); flex-shrink: 0;" />
                        </button>
                    </div>
                {/each}
            {:else}
                <!-- Original button (not yet resolved or no resolver) -->
                {@const isSupported = button.supported !== false}
                <div class="relative">
                    <!-- Main Button -->
                    <button
                        on:click={() => isSupported && handleButtonClick(button, index)}
                        disabled={state.loading || !isSupported}
                        class="w-full group flex items-center gap-4 p-5 rounded-[10px] transition-all duration-200 text-left disabled:cursor-not-allowed"
                        style="background: var(--bg-surface); border: 1px solid var(--border); opacity: {isSupported ? '1' : '0.45'};"
                        on:mouseenter={e => { if (!state.loading && isSupported) { const el = e.currentTarget as HTMLElement; el.style.background = 'var(--bg-raised)'; el.style.borderColor = 'var(--border-strong)'; }}}
                        on:mouseleave={e => { const el = e.currentTarget as HTMLElement; el.style.background = 'var(--bg-surface)'; el.style.borderColor = 'var(--border)'; }}
                    >
                        <!-- Icon -->
                        <div
                            class="flex-shrink-0 w-12 h-12 flex items-center justify-center rounded-[8px] transition-all"
                            style="border: 1px solid {isSupported ? (buttonColor ? buttonColor + '30' : 'var(--border)') : 'var(--border)'}; background: rgba(255,255,255,0.05);"
                        >
                            {#if !isSupported}
                                <Lock size={18} strokeWidth={1.75} style="color: var(--label-quaternary);" />
                            {:else if state.loading}
                                <Loader2 size={20} strokeWidth={1.75} class="animate-spin" style="color: var(--label-secondary);" />
                            {:else}
                                <svelte:component
                                    this={ButtonIconComponent}
                                    size={20}
                                    strokeWidth={1.75}
                                    style="color: {buttonColor || 'var(--label-secondary)'};"
                                />
                            {/if}
                        </div>

                        <!-- Content -->
                        <div class="flex-1 min-w-0">
                            <p class="text-[13px] font-semibold truncate" style="color: {isSupported ? 'var(--label-primary)' : 'var(--label-tertiary)'};">{buttonLabel}</p>
                            {#if !isSupported}
                                <p class="text-[12px] mt-0.5" style="color: var(--label-quaternary);">Not yet supported</p>
                            {:else if state.status}
                                <p class="text-[12px] mt-0.5" style="color: #32d74b;">{state.status}</p>
                            {:else if state.detectedHost?.supports_direct_download}
                                <p class="text-[12px] mt-0.5" style="color: var(--label-tertiary);">Direct download available</p>
                            {:else if state.detectedHost?.browser_only_reason}
                                <p class="text-[12px] mt-0.5" style="color: #ff9f0a;">{state.detectedHost.browser_only_reason}</p>
                            {:else}
                                <p class="text-[12px] mt-0.5" style="color: var(--label-tertiary);">
                                    {action === 'download_file' || action === 'smart_download' ? 'Click to download' : 'Click to open'}
                                </p>
                            {/if}
                        </div>

                        <!-- Arrow / Lock -->
                        {#if !isSupported}
                            <!-- no arrow for locked -->
                        {:else if action === 'open_link'}
                            <ExternalLink size={16} strokeWidth={1.75} style="color: var(--label-tertiary); flex-shrink: 0;" />
                        {:else}
                            <Download size={16} strokeWidth={1.75} style="color: var(--label-tertiary); flex-shrink: 0;" />
                        {/if}
                    </button>

                    <!-- Error Banner -->
                    {#if state.error}
                        <div class="mt-2 p-3 rounded-[8px]"
                             style="background: rgba(255,69,58,0.08); border: 1px solid rgba(255,69,58,0.2);">
                            <div class="flex items-start gap-2">
                                <AlertCircle size={14} style="color: #ff453a; flex-shrink: 0; margin-top: 1px;" />
                                <div class="flex-1 min-w-0">
                                    <p class="text-[12px] break-words" style="color: #ff453a;">{state.error}</p>
                                </div>
                                <button
                                    on:click={() => dismissError(index)}
                                    class="text-[12px] transition-colors"
                                    style="color: var(--label-tertiary);"
                                    on:mouseenter={e => { (e.currentTarget as HTMLElement).style.color = 'var(--label-primary)'; }}
                                    on:mouseleave={e => { (e.currentTarget as HTMLElement).style.color = 'var(--label-tertiary)'; }}
                                >
                                    ×
                                </button>
                            </div>

                            {#if state.fallbackUrl}
                                <button
                                    on:click={() => openFallback(index)}
                                    class="mt-2 w-full flex items-center justify-center gap-2 px-3 py-2 rounded-[6px] text-[12px] transition-colors"
                                    style="background: rgba(255,255,255,0.05); border: 1px solid var(--border); color: var(--label-secondary);"
                                    on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.09)'; }}
                                    on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.05)'; }}
                                >
                                    <Globe size={13} />
                                    Open in Browser
                                </button>
                            {/if}
                        </div>
                    {/if}
                </div>
            {/if}
        {/each}
    </div>

    <!-- Separator -->
    <div class="h-px mt-10" style="background: var(--border-subtle);"></div>
</div>

<!-- Install preflight modal -->
{#if pendingInstall || preflightLoading}
    <InstallConfirmModal
        {gameTitle}
        {coverUrl}
        preflight={preflightInfo}
        loading={preflightLoading}
        {sizeProbing}
        onConfirm={confirmInstall}
        onCancel={cancelInstall}
        onChangeDownloadDir={handleChangeDownloadDir}
        onChangeInstallDir={handleChangeInstallDir}
        {downloadDirOptions}
        {installDirOptions}
    />
{/if}

<!-- Source notice modal (download_start trigger) -->
{#if pendingNotice}
    <NoticeModal
        notice={pendingNotice}
        sourceId={sourceId}
        onConfirm={onNoticeConfirm}
        onCancel={onNoticeCancel}
    />
{/if}

<!-- Per-button warning confirmation -->
{#if pendingWarning}
    <div
        class="fixed inset-0 z-[500] flex items-center justify-center"
        style="background: rgba(0,0,0,0.6); backdrop-filter: blur(4px);"
        on:click|self={() => pendingWarning = null}
    >
        <div
            class="flex flex-col gap-4 p-6 rounded-[12px] max-w-sm w-full mx-4"
            style="background: var(--bg-elevated); border: 1px solid rgba(255,159,10,0.25);"
        >
            <!-- Header -->
            <div class="flex items-center gap-3">
                <div class="flex-shrink-0 w-8 h-8 flex items-center justify-center rounded-full"
                     style="background: rgba(255,159,10,0.12); border: 1px solid rgba(255,159,10,0.25);">
                    <AlertTriangle size={15} strokeWidth={2} style="color: #ff9f0a;" />
                </div>
                <p class="text-[14px] font-semibold" style="color: var(--label-primary);">Heads Up</p>
            </div>

            <!-- Message -->
            <p class="text-[13px] leading-relaxed" style="color: var(--label-secondary);">
                {pendingWarning.message}
            </p>

            <!-- Actions -->
            <div class="flex gap-2 justify-end">
                <button
                    on:click={() => pendingWarning = null}
                    class="px-4 py-2 rounded-[7px] text-[13px] font-medium transition-colors"
                    style="background: rgba(255,255,255,0.06); border: 1px solid var(--border); color: var(--label-secondary);"
                    on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.1)'; }}
                    on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.06)'; }}
                >
                    Cancel
                </button>
                <button
                    on:click={() => { const a = pendingWarning!.action; pendingWarning = null; a(); }}
                    class="px-4 py-2 rounded-[7px] text-[13px] font-medium transition-colors"
                    style="background: rgba(255,159,10,0.15); border: 1px solid rgba(255,159,10,0.3); color: #ff9f0a;"
                    on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,159,10,0.22)'; }}
                    on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,159,10,0.15)'; }}
                >
                    Continue Anyway
                </button>
            </div>
        </div>
    </div>
{/if}

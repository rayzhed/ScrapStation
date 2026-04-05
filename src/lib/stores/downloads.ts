import { writable, derived } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { DownloadGroup } from '$lib/types';

export type DownloadStatus = 'queued' | 'pending' | 'downloading' | 'paused' | 'completed' | 'failed' | 'cancelled';

/** Convert a raw backend error string into a user-friendly message. */
function friendlyDownloadError(error: string): string {
    if (error.startsWith('disk_space:')) return 'Not enough disk space';
    if (/network error|connection (reset|refused|timed? ?out)|timed? ?out/i.test(error)) return 'Network error — check your connection';
    if (/http error: 403|forbidden/i.test(error)) return 'Access denied (403)';
    if (/http error: 404|not found/i.test(error)) return 'File not found (404)';
    if (/http error: 5\d\d/i.test(error)) return 'Server error — try again later';
    if (/permission|access denied|cannot open|locked/i.test(error)) return 'File access error';
    // Strip internal prefixes but keep the message readable
    return error.replace(/^(Write error|Flush error|Network error): /, '').slice(0, 80) || 'Download failed';
}

export interface Download {
    id: string;
    game_title: string;
    file_name: string;
    url: string;
    source_id: string;
    host_label: string;
    host_color: string;
    status: DownloadStatus;
    downloaded_bytes: number;
    total_bytes: number;
    speed: number;
    file_path?: string;
    error?: string;
    started_at: number;
    completed_at?: number;
    is_resumable: boolean;
}

// Frontend-friendly interface (camelCase)
export interface DownloadDisplay {
    id: string;
    gameTitle: string;
    fileName: string;
    url: string;
    sourceId: string;
    hostLabel: string;
    hostColor: string;
    status: DownloadStatus;
    progress: number;
    downloadedBytes: number;
    totalBytes: number;
    speed: number;
    filePath?: string;
    error?: string;
    startedAt: number;
    completedAt?: number;
    isIndeterminate: boolean;
    isResumable: boolean;
}

// Convert backend format to frontend format
function toDisplayFormat(d: Download): DownloadDisplay {
    const progress = d.total_bytes > 0 ? (d.downloaded_bytes / d.total_bytes) * 100 : -1;
    return {
        id: d.id,
        gameTitle: d.game_title,
        fileName: d.file_name,
        url: d.url,
        sourceId: d.source_id,
        hostLabel: d.host_label,
        hostColor: d.host_color,
        status: d.status,
        progress,
        downloadedBytes: d.downloaded_bytes,
        totalBytes: d.total_bytes,
        speed: d.speed || 0,
        filePath: d.file_path,
        error: d.error,
        startedAt: d.started_at,
        completedAt: d.completed_at,
        isIndeterminate: d.total_bytes === 0 && d.status === 'downloading',
        isResumable: d.is_resumable
    };
}

// Store for all downloads
export const downloads = writable<DownloadDisplay[]>([]);

// Derived stores for filtering
export const activeDownloads = derived(downloads, $downloads =>
    $downloads.filter(d => d.status === 'downloading' || d.status === 'pending')
);

export const pausedDownloads = derived(downloads, $downloads =>
    $downloads.filter(d => d.status === 'paused')
);

export const completedDownloads = derived(downloads, $downloads =>
    $downloads.filter(d => d.status === 'completed')
);

export const failedDownloads = derived(downloads, $downloads =>
    $downloads.filter(d => d.status === 'failed')
);

// Stats
export const downloadStats = derived(downloads, $downloads => {
    const active = $downloads.filter(d => d.status === 'downloading');
    const paused = $downloads.filter(d => d.status === 'paused').length;
    const totalSpeed = active.reduce((sum, d) => sum + d.speed, 0);
    const completed = $downloads.filter(d => d.status === 'completed').length;
    const failed = $downloads.filter(d => d.status === 'failed').length;
    const pending = $downloads.filter(d => d.status === 'pending' || d.status === 'downloading').length;

    return {
        activeCount: active.length,
        pausedCount: paused,
        totalSpeed,
        completedCount: completed,
        failedCount: failed,
        pendingCount: pending
    };
});

// ========== DOWNLOAD GROUPING ==========

// Multi-part archive patterns
const MULTI_PART_PATTERNS = [
    /\.part\d+\.rar$/i,       // .part1.rar, .part2.rar
    /\.r\d{2}$/i,              // .r00, .r01, .r02
    /\.7z\.\d{3}$/i,           // .7z.001, .7z.002
    /\.zip\.\d{3}$/i,          // .zip.001, .zip.002
    /\.z\d{2}$/i,              // .z01, .z02
];

function isMultiPartArchive(filename: string): boolean {
    return MULTI_PART_PATTERNS.some(pattern => pattern.test(filename));
}

function getGroupKey(download: DownloadDisplay): string {
    return `${download.sourceId}_${download.gameTitle}`;
}

function determineGroupStatus(downloads: DownloadDisplay[]): DownloadGroup['status'] {
    const hasDownloading = downloads.some(d => d.status === 'downloading');
    const hasPending = downloads.some(d => d.status === 'pending');
    const hasQueued = downloads.some(d => d.status === 'queued');
    const hasPaused = downloads.some(d => d.status === 'paused');
    const hasFailed = downloads.some(d => d.status === 'failed');
    const allCompleted = downloads.every(d => d.status === 'completed');

    if (allCompleted) return 'completed';
    if (hasFailed && !hasDownloading && !hasPending && !hasQueued) return 'failed';
    if (hasDownloading) return 'downloading';
    if (hasPaused) return 'paused';
    if (hasQueued) return 'queued';
    if (hasPending) return 'pending';
    return 'pending';
}

// Derived store for grouped downloads by game
export const downloadGroups = derived(downloads, $downloads => {
    const groupMap = new Map<string, DownloadDisplay[]>();

    // Group downloads by game (source + title)
    for (const download of $downloads) {
        // Skip cancelled downloads
        if (download.status === 'cancelled') continue;

        const key = getGroupKey(download);
        const existing = groupMap.get(key) || [];
        existing.push(download);
        groupMap.set(key, existing);
    }

    // Convert to DownloadGroup array
    const groups: DownloadGroup[] = [];

    for (const [_key, groupDownloads] of groupMap) {
        // Sort downloads by filename for consistent ordering
        groupDownloads.sort((a, b) => a.fileName.localeCompare(b.fileName));

        const first = groupDownloads[0];
        const totalSize = groupDownloads.reduce((sum, d) => sum + d.totalBytes, 0);
        const downloadedSize = groupDownloads.reduce((sum, d) => sum + d.downloadedBytes, 0);
        // If any active/pending download has no known size yet, treat the whole
        // group as indeterminate (-1) so the UI shows "Starting..." instead of 100%.
        const hasActiveIndeterminate = groupDownloads.some(d =>
            (d.status === 'downloading' || d.status === 'pending') && d.totalBytes === 0
        );
        const progress = hasActiveIndeterminate ? -1 : (totalSize > 0 ? (downloadedSize / totalSize) * 100 : 0);
        const status = determineGroupStatus(groupDownloads);
        const isMultiPart = groupDownloads.length > 1 || groupDownloads.some(d => isMultiPartArchive(d.fileName));
        const allCompleted = groupDownloads.every(d => d.status === 'completed');
        const hasArchives = groupDownloads.some(d =>
            d.filePath && /\.(rar|7z|zip)$/i.test(d.filePath)
        );

        groups.push({
            gameId: getGroupKey(first),
            gameTitle: first.gameTitle,
            sourceId: first.sourceId,
            downloads: groupDownloads,
            totalSize,
            downloadedSize,
            progress,
            status,
            canExtract: allCompleted && hasArchives,
            isMultiPart,
        });
    }

    // Sort groups: downloading first, then pending, then completed
    const statusOrder: Record<string, number> = {
        'downloading': 0,
        'extracting': 1,
        'pending': 2,
        'paused': 3,
        'completed': 4,
        'failed': 5,
    };

    groups.sort((a, b) => {
        const orderDiff = (statusOrder[a.status] ?? 99) - (statusOrder[b.status] ?? 99);
        if (orderDiff !== 0) return orderDiff;
        // Same status: sort by game title
        return a.gameTitle.localeCompare(b.gameTitle);
    });

    return groups;
});

// Get archive paths for a completed download group
export function getArchivePaths(group: DownloadGroup): string[] {
    return group.downloads
        .filter(d => d.status === 'completed' && d.filePath)
        .map(d => d.filePath!)
        .filter(path => /\.(rar|7z|zip)$/i.test(path));
}

// Load downloads from backend (persistent storage)
export async function loadDownloads(): Promise<void> {
    try {
        const data = await invoke<Download[]>('get_downloads');
        downloads.set(data.map(toDisplayFormat));
    } catch (e) {
    }
}

// Update download progress (local update, used by event listeners)
export function updateDownloadProgress(id: string, update: Partial<DownloadDisplay>) {
    downloads.update(list =>
        list.map(d => d.id === id ? { ...d, ...update } : d)
    );
}

// Pause a download
export async function pauseDownload(id: string): Promise<void> {
    try {
        await invoke('pause_download', { id });
        updateDownloadProgress(id, { status: 'paused' });
    } catch (e) {
        throw e;
    }
}

// Resume a download
export async function resumeDownload(id: string): Promise<void> {
    try {
        await invoke('resume_download', { id });
        updateDownloadProgress(id, { status: 'pending' });
    } catch (e) {
        throw e;
    }
}

// Cancel a download
export async function cancelDownload(id: string): Promise<void> {
    try {
        await invoke('cancel_download', { id });
        updateDownloadProgress(id, { status: 'cancelled' });
        // Cancelled download frees the slot — advance queue
        _queueProcessing = false;
        await _processQueue();
    } catch (e) {
        throw e;
    }
}

// Remove download from list
export async function removeDownload(id: string): Promise<void> {
    try {
        await invoke('remove_download', { id });
        downloads.update(list => list.filter(d => d.id !== id));
    } catch (e) {
        throw e;
    }
}

// Clear completed/failed downloads
export async function clearFinished(): Promise<void> {
    try {
        await invoke('clear_finished_downloads');
        downloads.update(list => list.filter(d =>
            d.status === 'downloading' || d.status === 'pending' || d.status === 'paused'
        ));
    } catch (e) {
        throw e;
    }
}

// Get download folder path
export async function getDownloadFolder(): Promise<string> {
    return invoke<string>('get_download_folder_path');
}

// Open download folder
export async function openDownloadFolder(): Promise<void> {
    await invoke('open_download_folder');
}

// Open file location
export async function openFileLocation(filePath: string): Promise<void> {
    await invoke('open_file_location', { filePath });
}

// Format bytes to human readable
export function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// Format speed to human readable
export function formatSpeed(bytesPerSecond: number): string {
    return formatBytes(bytesPerSecond) + '/s';
}

// Format duration
export function formatDuration(ms: number): string {
    if (!ms || ms < 0) return '--';
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (hours > 0) {
        return `${hours}h ${minutes % 60}m`;
    }
    if (minutes > 0) {
        return `${minutes}m ${seconds % 60}s`;
    }
    return `${seconds}s`;
}

// Estimate time remaining
export function formatETA(downloadedBytes: number, totalBytes: number, speed: number): string {
    if (speed <= 0 || totalBytes <= 0) return '--';
    const remaining = totalBytes - downloadedBytes;
    const seconds = remaining / speed;
    return formatDuration(seconds * 1000);
}

// ========== DOWNLOAD QUEUE ==========

interface QueuedAction {
    downloadId: string;
    url: string;
    sourceId: string;
    filenameHint: string | null;
    preResolvedUrl: string | null;
    preCookies: string | null;
    downloadDir: string | null;
    installDir: string | null;
}

const _queue: QueuedAction[] = [];
let _queueProcessing = false;

/** Returns true if any download OTHER than excludeId is currently in-flight. */
function hasActiveDownload(excludeId?: string): boolean {
    let active = false;
    const unsub = downloads.subscribe(list => {
        active = list.some(d =>
            d.id !== excludeId &&
            (d.status === 'downloading' || d.status === 'pending')
        );
    });
    unsub();
    return active;
}

/**
 * Start a download immediately (internal).
 * Registers the status change and calls smart_download.
 */
async function _startDownload(action: QueuedAction): Promise<void> {
    _queueProcessing = true;
    await invoke('update_download_status', {
        id: action.downloadId,
        status: 'downloading',
        error: null,
        filePath: null,
    }).catch(() => {});

    await invoke('smart_download', {
        url: action.url,
        sourceId: action.sourceId,
        filenameHint: action.filenameHint,
        downloadId: action.downloadId,
        preResolvedUrl: action.preResolvedUrl ?? null,
        preCookies: action.preCookies ?? null,
        downloadDir: action.downloadDir ?? null,
        installDir: action.installDir ?? null,
    }).catch(() => {
        _queueProcessing = false;
    });
    // _queueProcessing is reset when download-complete or download-error fires
}

/** Advance the queue: start next queued download, if any. */
async function _processQueue(): Promise<void> {
    if (_queueProcessing) return;
    const next = _queue.shift();
    if (!next) return;
    await _startDownload(next);
}

/**
 * Enqueue a download.
 * If nothing is downloading right now, starts immediately.
 * Otherwise adds it to the queue with 'queued' status.
 */
export async function enqueueDownload(params: {
    downloadId: string;
    url: string;
    sourceId: string;
    filenameHint?: string | null;
    preResolvedUrl?: string | null;
    preCookies?: string | null;
    downloadDir?: string | null;
    installDir?: string | null;
}): Promise<void> {
    const action: QueuedAction = {
        downloadId: params.downloadId,
        url: params.url,
        sourceId: params.sourceId,
        filenameHint: params.filenameHint ?? null,
        preResolvedUrl: params.preResolvedUrl ?? null,
        preCookies: params.preCookies ?? null,
        downloadDir: params.downloadDir ?? null,
        installDir: params.installDir ?? null,
    };

    if (!hasActiveDownload(action.downloadId) && !_queueProcessing) {
        await _startDownload(action);
    } else {
        // Mark as queued in backend so the UI can show queue position
        await invoke('update_download_status', {
            id: action.downloadId,
            status: 'queued',
            error: null,
            filePath: null,
        }).catch(() => {});
        _queue.push(action);
    }
}

// ========== EVENT LISTENERS ==========

// Initialize Tauri event listeners
export async function initDownloadEvents(): Promise<void> {
    // Load persisted downloads first
    await loadDownloads();

    // Listen for download progress events from backend
    await listen<{
        id: string;
        downloaded_bytes: number;
        total_bytes: number;
        speed: number;
    }>('download-progress', (event) => {
        const { id, downloaded_bytes, total_bytes, speed } = event.payload;
        const progress = total_bytes > 0 ? (downloaded_bytes / total_bytes) * 100 : -1;
        const hasRealProgress = total_bytes > 0 && downloaded_bytes > 0;

        downloads.update(list => list.map(d => {
            if (d.id !== id) return d;
            // Never override a terminal or paused state with 'downloading'.
            // Stale progress events can arrive after cancel/fail/pause.
            if (d.status === 'cancelled' || d.status === 'failed' ||
                d.status === 'completed' || d.status === 'paused') return d;
            return {
                ...d,
                status: 'downloading' as DownloadStatus,
                downloadedBytes: downloaded_bytes,
                totalBytes: total_bytes,
                speed,
                progress,
                isIndeterminate: !hasRealProgress
            };
        }));
    });

    // Listen for download completion → advance queue
    await listen<{
        id: string;
        file_path: string;
    }>('download-complete', async (event) => {
        const { id, file_path } = event.payload;
        _queueProcessing = false;
        downloads.update(list => list.map(d => {
            if (d.id !== id) return d;
            if (d.status === 'cancelled') return d;
            return { ...d, status: 'completed' as DownloadStatus, progress: 100, filePath: file_path, completedAt: Date.now(), isIndeterminate: false };
        }));
        // Start next queued download
        await _processQueue();
    });

    // Listen for download errors → also advance queue so it doesn't stall
    await listen<{
        id: string;
        error: string;
    }>('download-error', async (event) => {
        const { id, error } = event.payload;
        _queueProcessing = false;
        const friendlyError = friendlyDownloadError(error);
        downloads.update(list => list.map(d => {
            if (d.id !== id) return d;
            if (d.status === 'cancelled') return d;
            return { ...d, status: 'failed' as DownloadStatus, error: friendlyError };
        }));
        await _processQueue();
    });

    // Listen for full downloads update from backend
    await listen<Download[]>('downloads-updated', async (event) => {
        const data = event.payload.map(toDisplayFormat);
        downloads.set(data);

        // If the queue lock is stuck (no active/pending downloads remain but
        // _queueProcessing is still true), release it and advance the queue.
        // This catches deletes, unexpected failures, and edge cases not covered
        // by the individual complete/error handlers.
        if (_queueProcessing) {
            const hasActive = data.some(d => d.status === 'downloading' || d.status === 'pending');
            const hasPaused = data.some(d => d.status === 'paused');
            if (!hasActive && !hasPaused) {
                _queueProcessing = false;
                await _processQueue();
            }
        }
    });

    // Listen for filename updates (when actual filename is determined from server)
    await listen<{
        id: string;
        file_name: string;
    }>('download-filename-updated', (event) => {
        const { id, file_name } = event.payload;
        updateDownloadProgress(id, { fileName: file_name });
    });
}

// Register a new download with the backend tracker
export function addDownload(params: {
    id?: string;
    gameTitle: string;
    fileName: string;
    url: string;
    sourceId: string;
    hostLabel: string;
    hostColor: string;
    isIndeterminate?: boolean;
}): string {
    const id = params.id || `dl_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;

    // Register with backend (async, fire-and-forget for now)
    invoke('register_download', {
        id,
        gameTitle: params.gameTitle,
        fileName: params.fileName,
        url: params.url,
        sourceId: params.sourceId,
        hostLabel: params.hostLabel,
        hostColor: params.hostColor
    }).catch(() => {});

    return id;
}

// Update download status - syncs with backend
export function startDownload(id: string): void {
    invoke('update_download_status', {
        id,
        status: 'downloading',
        error: null,
        filePath: null
    }).catch(() => {});
}

export function completeDownload(id: string, filePath?: string, _fileSize?: number): void {
    invoke('update_download_status', {
        id,
        status: 'completed',
        error: null,
        filePath: filePath || null
    }).catch(() => {});
}

export function failDownload(id: string, error: string): void {
    invoke('update_download_status', {
        id,
        status: 'failed',
        error,
        filePath: null
    }).catch(() => {});
}

// Legacy aliases for backward compatibility
export const clearCompleted = clearFinished;

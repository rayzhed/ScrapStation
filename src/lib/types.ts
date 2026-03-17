export interface GameTag {
    id: string;
    label: string;
    color?: string;
    background?: string;
    icon?: string;
    priority: number;
    style?: string;
}

export interface GameCard {
    // Required fields
    title: string;
    cover_url: string;
    game_url: string;
    tags: GameTag[];

    // All other fields are dynamic and accessed via index signature
    // Common optional fields for convenience typing:
    description?: string;
    genre?: string;
    size?: string;
    author?: string;
    is_online?: boolean;
    release_date?: string;
    platform?: string;
    game_modes?: string;

    // Allow any additional dynamic fields from config
    [key: string]: any;
}

export interface Source {
    id: string;
    name: string;
    description?: string;
    icon: string;
    color: string;
}

export interface CacheEntry<T> {
    data: T;
    timestamp: number;
}

// Smart Download Types
export interface DetectedHost {
    host_id: string;
    label: string;
    icon: string | null;
    color: string | null;
    supports_direct_download: boolean;
    browser_only_reason: string | null;
}

export interface SmartDownloadResult {
    success: boolean;
    file_path: string | null;
    file_size: number | null;
    error: DownloadError | null;
    fallback_url: string | null;
}

export interface DownloadError {
    error_type: DownloadErrorType;
    message: string;
    recoverable: boolean;
}

export type DownloadErrorType =
    | 'captcha_required'
    | 'premium_only'
    | 'geo_blocked'
    | 'rate_limited'
    | 'network_error'
    | 'resolution_failed'
    | 'unsupported_host'
    | 'file_not_found'
    | 'browser_required'
    | 'unknown';

export interface DownloadButton {
    label: string;
    url: string;
    icon: string;
    style: string;
    action: 'open_link' | 'download_file' | 'smart_download';
    resolve_link?: boolean;
    smart_download?: boolean;
    resolver?: string;
    supported?: boolean;  // false = locked (not yet supported by config author)
    warning?: string;     // shown as confirmation dialog before proceeding
}

// ========== LIBRARY TYPES ==========

export type LibraryGameStatus = 'downloading' | 'extracting' | 'ready' | 'corrupted';

export type ExeType = 'main' | 'launcher' | 'tool' | 'installer' | 'redistributable' | 'unknown';

export interface GameExecutable {
    path: string;
    name: string;
    score: number;
    exe_type: ExeType;
}

export interface LibraryGame {
    id: string;
    source_slug: string;
    source_game_id: string;
    title: string;
    cover_url?: string;
    cover_path?: string;
    install_path: string;
    install_size: number;
    status: LibraryGameStatus;
    installed_at: number;
    last_played?: number;
    total_playtime: number;
    executables: GameExecutable[];
    primary_exe?: string;
    archive_password?: string;
    download_ids: string[];
}

export interface ExtractionProgress {
    game_id: string;
    current_file: string;
    files_done: number;
    files_total: number;
    bytes_done: number;
    bytes_total: number;
    current_archive: number;
    total_archives: number;
}

// ========== DOWNLOAD GROUPING TYPES ==========

export interface DownloadGroup {
    gameId: string;
    gameTitle: string;
    sourceId: string;
    coverUrl?: string;
    downloads: DownloadDisplay[];
    totalSize: number;
    downloadedSize: number;
    progress: number;
    status: 'pending' | 'downloading' | 'paused' | 'completed' | 'failed' | 'extracting';
    canExtract: boolean;
    isMultiPart: boolean;
    libraryGameId?: string;
}

export interface DownloadDisplay {
    id: string;
    gameTitle: string;
    fileName: string;
    url: string;
    sourceId: string;
    hostLabel: string;
    hostColor: string;
    status: 'pending' | 'downloading' | 'paused' | 'completed' | 'failed' | 'cancelled';
    downloadedBytes: number;
    totalBytes: number;
    speed: number;
    progress: number;
    isIndeterminate: boolean;
    filePath?: string;
    error?: string;
    startedAt: number;
    completedAt?: number;
    isResumable: boolean;
}
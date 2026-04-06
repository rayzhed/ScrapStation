<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { listen, type UnlistenFn } from '@tauri-apps/api/event';
    import { open as openDialog } from '@tauri-apps/plugin-dialog';
    import { sources } from '$lib/stores/sources';
    import { storageConfig, loadStorageConfig, setDataRoot, getKnownLibraryLocations, addLibraryLocation, removeLibraryLocation, type KnownLibraryLocation } from '$lib/stores/appSettings';
    import { downloadStats } from '$lib/stores/downloads';
    import { Settings, ChevronDown, ChevronRight, Save, RotateCcw, AlertCircle, Check, FolderOpen, ExternalLink, HardDrive, Wrench, CheckCircle, XCircle, Plus, Trash2 } from 'lucide-svelte';
    import * as LucideIcons from 'lucide-svelte';
    import { onMount, onDestroy } from 'svelte';
    import SettingSectionRenderer from './SettingSectionRenderer.svelte';
    import LegacySettingField from './settings/LegacySettingField.svelte';

    interface AuthState {
        isLoggedIn: boolean;
        username?: string;
        isLoading: boolean;
        error?: string;
        success?: string;
    }

    interface SettingDefinition {
        id: string;
        type: 'text' | 'textarea' | 'toggle' | 'select' | 'number' | 'password' | 'auth';
        label: string;
        description?: string;
        placeholder?: string;
        default?: any;
        required?: boolean;
        secret?: boolean;
        options?: { value: string; label: string }[];
        config?: any;
    }

    interface SettingSection {
        id: string;
        title: string;
        icon?: string;
        description?: string;
        components: any[];
    }

    interface SourceSettings {
        sourceId: string;
        settingSections?: SettingSection[];
        schema: SettingDefinition[];
        values: Record<string, any>;
        editValues: Record<string, any>;
        expanded: boolean;
        hasChanges: boolean;
        isSaving: boolean;
        showSecrets: Record<string, boolean>;
        authState?: AuthState;
    }

    interface AuthResult {
        success: boolean;
        cookies?: string;
        username?: string;
        error?: string;
        source_id: string;
    }

    let sourceSettings: SourceSettings[] = [];
    let loading = false;
    let sourcesFolderPath = '';

    // Storage configuration
    let storageSaving = false;
    let storageRootChanged = false;
    let showRootPicker = false;

    /** Derive a data-root suggestion from a known library path by stripping the ScrapStation\Library suffix. */
    function libraryPathToRoot(libraryPath: string): string {
        return libraryPath.replace(/[/\\]ScrapStation[/\\]Library[/\\]?$/i, '');
    }

    /** Unique suggested data roots derived from all known library locations (excluding current active). */
    $: suggestedRoots = (() => {
        const currentRoot = $storageConfig?.data_root ?? null;
        const seen = new Set<string>();
        return knownLibraryLocations
            .filter(l => !l.is_current)
            .map(l => ({ path: libraryPathToRoot(l.path), label: l.label }))
            .filter(r => {
                const key = r.path.toLowerCase();
                if (seen.has(key)) return false;
                seen.add(key);
                if (currentRoot && r.path.toLowerCase() === currentRoot.toLowerCase()) return false;
                return r.path.length > 0;
            });
    })();
    let knownLibraryLocations: KnownLibraryLocation[] = [];
    let addingLocation = false;
    let locationError: string | null = null;

    async function loadLibraryLocations() {
        try {
            knownLibraryLocations = await getKnownLibraryLocations();
        } catch (e) {
            console.error('[Settings] Failed to load library locations:', e);
        }
    }

    async function handleAddLibraryLocation() {
        const result = await openDialog({ directory: true, title: 'Add Library Folder' });
        if (!result) return;
        locationError = null;
        addingLocation = true;
        try {
            await addLibraryLocation(result as string);
            await loadLibraryLocations();
        } catch (e) {
            locationError = String(e);
            setTimeout(() => { locationError = null; }, 6000);
        } finally {
            addingLocation = false;
        }
    }

    async function handleRemoveLibraryLocation(path: string) {
        try {
            await removeLibraryLocation(path);
            await loadLibraryLocations();
        } catch (e) {
            locationError = String(e);
            setTimeout(() => { locationError = null; }, 6000);
        }
    }

    // Recovery
    interface RecoveryResult {
        success: boolean;
        message: string;
        details: string[];
    }
    let migrationRunning = false;
    let migrationResult: RecoveryResult | null = null;
    let pathFixRunning = false;
    let pathFixResult: RecoveryResult | null = null;

    async function runMigration() {
        migrationRunning = true;
        migrationResult = null;
        try {
            migrationResult = await invoke<RecoveryResult>('run_appdata_migration');
        } catch (e) {
            migrationResult = { success: false, message: String(e), details: [] };
        } finally {
            migrationRunning = false;
        }
    }

    async function runFixPaths() {
        pathFixRunning = true;
        pathFixResult = null;
        try {
            pathFixResult = await invoke<RecoveryResult>('fix_broken_paths');
        } catch (e) {
            pathFixResult = { success: false, message: String(e), details: [] };
        } finally {
            pathFixRunning = false;
        }
    }

    async function browseDataRoot() {
        const result = await openDialog({ directory: true, title: 'Choose Data Root Folder' });
        if (result) {
            storageSaving = true;
            try {
                await setDataRoot(result as string);
                storageRootChanged = true;
                await loadLibraryLocations();
            } finally {
                storageSaving = false;
            }
        }
    }

    async function resetDataRoot() {
        storageSaving = true;
        try {
            await setDataRoot(null);
            storageRootChanged = true;
            await loadLibraryLocations();
        } finally {
            storageSaving = false;
        }
    }

    async function openSourcesFolder() {
        await invoke('open_sources_folder');
    }

    async function loadSourcesFolderPath() {
        try {
            sourcesFolderPath = await invoke<string>('get_sources_folder_path');
        } catch {
            sourcesFolderPath = '';
        }
    }

    let hasLoaded = false;
    let ssoAuthUnlisten: UnlistenFn | null = null;
    let activeSection = 'storage';
    let scrollContainer: HTMLElement;

    function scrollToSection(id: string) {
        activeSection = id;
        const el = document.getElementById(id);
        if (el && scrollContainer) {
            scrollContainer.scrollTo({ top: el.offsetTop - 24, behavior: 'smooth' });
        }
    }

    function getIconComponent(iconName: string) {
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        return (LucideIcons as Record<string, any>)[pascalCase] || LucideIcons.Gamepad2;
    }

    onMount(async () => {
        await setupSsoAuthListener();
        await loadSourcesFolderPath();
        await loadStorageConfig();
        await loadLibraryLocations();
        await loadAllSettings();

        // Track active section via scroll position
        const observer = new IntersectionObserver(
            (entries) => {
                for (const entry of entries) {
                    if (entry.isIntersecting) activeSection = entry.target.id;
                }
            },
            { root: scrollContainer, rootMargin: '-20% 0px -60% 0px', threshold: 0 }
        );
        // Observe after a tick so IDs are rendered
        setTimeout(() => {
            ['section-storage', 'section-sources', 'section-source-settings', 'section-recovery'].forEach(id => {
                const el = document.getElementById(id);
                if (el) observer.observe(el);
            });
        }, 100);

        return () => observer.disconnect();
    });

    onDestroy(() => {
        if (ssoAuthUnlisten) ssoAuthUnlisten();
    });

    async function setupSsoAuthListener() {
        ssoAuthUnlisten = await listen<AuthResult>('auth-complete', async (event) => {
            const result = event.payload;
            sourceSettings = sourceSettings.map(s => {
                if (s.sourceId !== result.source_id || !s.authState) return s;
                if (result.success) {
                    return { ...s, authState: { ...s.authState, isLoading: false, isLoggedIn: true, username: result.username, success: 'Successfully logged in!', error: undefined } };
                } else {
                    const isCancelled = result.error === 'Authentication cancelled';
                    return { ...s, authState: { ...s.authState, isLoading: false, error: isCancelled ? undefined : (result.error || 'Authentication failed'), success: undefined } };
                }
            });
            if (result.success) setTimeout(() => clearAuthMessage(result.source_id, 'success'), 3000);
        });
    }

    function clearAuthMessage(sourceId: string, field: 'success' | 'error') {
        sourceSettings = sourceSettings.map(s => {
            if (s.sourceId !== sourceId || !s.authState) return s;
            return { ...s, authState: { ...s.authState, [field]: undefined } };
        });
    }

    async function refreshAllAuthStatus() {
        for (const source of sourceSettings) {
            if (source.authState) await refreshAuthStatusSilent(source.sourceId);
        }
    }

    async function refreshAuthStatusSilent(sourceId: string) {
        try {
            const result = await invoke<{ success: boolean; username?: string }>('get_auth_status', { sourceId });
            sourceSettings = sourceSettings.map(s => {
                if (s.sourceId !== sourceId || !s.authState) return s;
                return { ...s, authState: { ...s.authState, isLoggedIn: result.success, username: result.username } };
            });
        } catch (e) {
        }
    }

    async function loadAllSettings() {
        loading = true;
        const settings: SourceSettings[] = [];

        for (const source of $sources) {
            try {
                const [settingSections, schema, values] = await Promise.all([
                    invoke<SettingSection[]>('get_setting_sections', { sourceId: source.id }).catch(() => []),
                    invoke<SettingDefinition[]>('get_source_settings_schema', { sourceId: source.id }).catch(() => []),
                    invoke<Record<string, any>>('get_source_settings_values', { sourceId: source.id })
                ]);

                if (settingSections && settingSections.length > 0) {
                    settings.push({ sourceId: source.id, settingSections, schema: [], values, editValues: {}, expanded: true, hasChanges: false, isSaving: false, showSecrets: {} });
                } else if (schema && schema.length > 0) {
                    const editValues: Record<string, any> = {};
                    const showSecrets: Record<string, boolean> = {};
                    for (const setting of schema) {
                        editValues[setting.id] = values[setting.id] ?? setting.default ?? getDefaultForType(setting.type);
                        if (setting.secret) showSecrets[setting.id] = false;
                    }
                    const authSetting = schema.find(s => s.type === 'auth');
                    const authState = authSetting ? await initAuthState(source.id) : undefined;
                    settings.push({ sourceId: source.id, schema, values, editValues, expanded: true, hasChanges: false, isSaving: false, showSecrets, authState });
                }
            } catch (e) {
            }
        }

        if (settings.length > 0) settings[0].expanded = true;
        sourceSettings = settings;
        loading = false;
        hasLoaded = true;
    }

    function getDefaultForType(type: string): any {
        switch (type) {
            case 'toggle': return false;
            case 'number': return 0;
            default: return '';
        }
    }

    function toggleExpanded(sourceId: string) {
        sourceSettings = sourceSettings.map(s =>
            s.sourceId === sourceId ? { ...s, expanded: !s.expanded } : s
        );
    }

    function updateValue(sourceId: string, settingId: string, value: any) {
        sourceSettings = sourceSettings.map(s => {
            if (s.sourceId !== sourceId) return s;
            const newEditValues = { ...s.editValues, [settingId]: value };
            const hasChanges = Object.keys(newEditValues).some(
                key => JSON.stringify(newEditValues[key]) !== JSON.stringify(s.values[key] ?? getDefaultForType(s.schema.find(def => def.id === key)?.type || 'text'))
            );
            return { ...s, editValues: newEditValues, hasChanges };
        });
    }

    function toggleSecret(sourceId: string, settingId: string) {
        sourceSettings = sourceSettings.map(s =>
            s.sourceId === sourceId
                ? { ...s, showSecrets: { ...s.showSecrets, [settingId]: !s.showSecrets[settingId] } }
                : s
        );
    }

    async function saveSettings(sourceId: string) {
        const source = sourceSettings.find(s => s.sourceId === sourceId);
        if (!source) return;
        sourceSettings = sourceSettings.map(s => s.sourceId === sourceId ? { ...s, isSaving: true } : s);
        try {
            await invoke('set_source_settings', { sourceId, values: source.editValues });
            sourceSettings = sourceSettings.map(s =>
                s.sourceId === sourceId
                    ? { ...s, values: { ...source.editValues }, hasChanges: false, isSaving: false }
                    : s
            );
        } catch (e) {
            sourceSettings = sourceSettings.map(s => s.sourceId === sourceId ? { ...s, isSaving: false } : s);
        }
    }

    function resetSettings(sourceId: string) {
        sourceSettings = sourceSettings.map(s => {
            if (s.sourceId !== sourceId) return s;
            const editValues: Record<string, any> = {};
            for (const setting of s.schema) {
                editValues[setting.id] = s.values[setting.id] ?? setting.default ?? getDefaultForType(setting.type);
            }
            return { ...s, editValues, hasChanges: false };
        });
    }

    function getSource(sourceId: string) {
        return $sources.find(s => s.id === sourceId);
    }

    function hasConfiguredSettings(source: SourceSettings): boolean {
        if (source.settingSections && source.settingSections.length > 0) {
            const cookies = source.values['cookies'];
            return cookies !== undefined && cookies !== null && cookies !== '';
        }
        return source.schema.some(setting => {
            if (setting.type === 'auth') return source.authState?.isLoggedIn ?? false;
            const value = source.values[setting.id];
            if (value === undefined || value === null || value === '') return false;
            return JSON.stringify(value) !== JSON.stringify(setting.default ?? getDefaultForType(setting.type));
        });
    }

    async function initAuthState(sourceId: string): Promise<AuthState> {
        const authState: AuthState = { isLoggedIn: false, isLoading: false };
        try {
            const result = await invoke<{ success: boolean; username?: string }>('get_auth_status', { sourceId });
            authState.isLoggedIn = result.success;
            authState.username = result.username;
        } catch (e) {
        }
        return authState;
    }

    function setAuthLoading(sourceId: string, isLoading: boolean) {
        sourceSettings = sourceSettings.map(s =>
            s.sourceId === sourceId && s.authState
                ? { ...s, authState: { ...s.authState, isLoading, error: undefined, success: undefined } }
                : s
        );
    }

    async function handleLogout(sourceId: string) {
        setAuthLoading(sourceId, true);
        try {
            await invoke('logout', { sourceId });
            sourceSettings = sourceSettings.map(s => {
                if (s.sourceId !== sourceId || !s.authState) return s;
                return { ...s, authState: { ...s.authState, isLoading: false, isLoggedIn: false, username: undefined, success: 'Logged out successfully' } };
            });
            setTimeout(() => clearAuthMessage(sourceId, 'success'), 3000);
        } catch (e) {
            sourceSettings = sourceSettings.map(s =>
                s.sourceId === sourceId && s.authState
                    ? { ...s, authState: { ...s.authState, isLoading: false, error: String(e) } }
                    : s
            );
        }
    }

    async function handleSsoLogin(sourceId: string, providerId: string) {
        setAuthLoading(sourceId, true);
        try {
            await invoke('open_auth_window', { sourceId, providerId });
        } catch (e) {
            sourceSettings = sourceSettings.map(s =>
                s.sourceId === sourceId && s.authState
                    ? { ...s, authState: { ...s.authState, isLoading: false, error: String(e) } }
                    : s
            );
        }
    }

    async function refreshAuthStatus(sourceId: string) {
        setAuthLoading(sourceId, true);
        try {
            let result: { success: boolean; username?: string; error?: string };
            try {
                result = await invoke<typeof result>('extract_webview_cookies', { sourceId });
            } catch {
                result = await invoke<typeof result>('get_auth_status', { sourceId });
            }
            sourceSettings = sourceSettings.map(s => {
                if (s.sourceId !== sourceId || !s.authState) return s;
                return { ...s, authState: { ...s.authState, isLoading: false, isLoggedIn: result.success, username: result.username, success: result.success ? 'Logged in!' : undefined, error: !result.success && result.error ? result.error : undefined } };
            });
            if (result.success) setTimeout(() => clearAuthMessage(sourceId, 'success'), 3000);
        } catch (e) {
            sourceSettings = sourceSettings.map(s =>
                s.sourceId === sourceId && s.authState
                    ? { ...s, authState: { ...s.authState, isLoading: false, error: String(e) } }
                    : s
            );
        }
    }
</script>

<svelte:window on:click={() => { showRootPicker = false; }} />

<div class="flex h-full overflow-hidden">

    <!-- ── Left nav ────────────────────────────────────────────────────────── -->
    <div class="flex flex-col shrink-0 overflow-y-auto"
         style="width: 210px; background: var(--bg-sidebar); border-right: 1px solid var(--border-subtle);">
        <nav class="p-3 flex flex-col gap-0.5">
            <p class="nav-label">App</p>
            <button class="nav-item" class:nav-active={activeSection === 'section-storage'}
                    on:click={() => scrollToSection('section-storage')}>
                <HardDrive size={14} /> Storage
            </button>
            <button class="nav-item" class:nav-active={activeSection === 'section-sources'}
                    on:click={() => scrollToSection('section-sources')}>
                <FolderOpen size={14} /> Sources
            </button>
            <button class="nav-item" class:nav-active={activeSection === 'section-recovery'}
                    on:click={() => scrollToSection('section-recovery')}>
                <Wrench size={14} /> Recovery
            </button>

            {#if sourceSettings.length > 0}
                <p class="nav-label" style="margin-top: 16px;">Source Settings</p>
                {#each sourceSettings as s}
                    {@const src = getSource(s.sourceId)}
                    {#if src}
                        {@const Icon = getIconComponent(src.icon)}
                        <button class="nav-item"
                                class:nav-active={activeSection === 'section-source-settings'}
                                on:click={() => scrollToSection('section-source-settings')}>
                            <svelte:component this={Icon} size={13} style="color: {src.color}; flex-shrink: 0;" />
                            <span class="flex-1 truncate">{src.name}</span>
                            {#if hasConfiguredSettings(s)}
                                <span style="width:6px;height:6px;border-radius:50%;background:#32d74b;flex-shrink:0;"></span>
                            {:else if s.hasChanges}
                                <span style="width:6px;height:6px;border-radius:50%;background:#ff9f0a;flex-shrink:0;"></span>
                            {/if}
                        </button>
                    {/if}
                {/each}
            {/if}
        </nav>
    </div>

    <!-- ── Right content ───────────────────────────────────────────────────── -->
    <div class="flex-1 overflow-y-auto" bind:this={scrollContainer}>
    <div style="padding: 36px 40px 96px;">

    <!-- ══ Storage ══════════════════════════════════════════════════════════ -->
    <section id="section-storage">
        <div class="section-header">
            <HardDrive size={14} style="color: #0a84ff; flex-shrink: 0; margin-top: 2px;" />
            <div>
                <h2 class="section-title">Storage</h2>
                <p class="section-desc">Where ScrapStation stores your downloads and game library.</p>
            </div>
        </div>

        <div class="settings-card">
            <div class="settings-row">
                <div class="settings-row-label">
                    <span class="row-title">Data root</span>
                    <span class="row-desc">
                        {#if $storageConfig}
                            {#if $storageConfig.data_root}
                                <span class="path-text" title={$storageConfig.data_root}>{$storageConfig.data_root}</span>
                            {:else}
                                Default location <span class="path-text" style="display:inline;">(AppData)</span>
                            {/if}
                        {:else}
                            Loading…
                        {/if}
                    </span>
                </div>
                <div class="flex items-center gap-2">
                    <div style="position: relative;">
                        <button class="btn-secondary"
                                on:click|stopPropagation={() => showRootPicker = !showRootPicker}
                                disabled={storageSaving || ($downloadStats?.activeCount ?? 0) > 0}
                                title={($downloadStats?.activeCount ?? 0) > 0 ? 'Pause downloads before changing storage location' : 'Change data root'}>
                            <FolderOpen size={12} /> Change
                        </button>
                        {#if showRootPicker}
                            <div class="move-dropdown" style="right: 0; left: auto; min-width: 220px;">
                                {#if suggestedRoots.length > 0}
                                    <div class="move-dropdown-header">Switch to location</div>
                                    {#each suggestedRoots as root}
                                        <button class="move-option" on:click={async () => { showRootPicker = false; storageSaving = true; try { await setDataRoot(root.path); await loadLibraryLocations(); storageRootChanged = true; } catch(e) { console.error(e); } finally { storageSaving = false; } }}>
                                            <span style="font-size: 11px; font-weight: 600; color: var(--label-primary);">{root.path}</span>
                                            <span style="font-size: 10px; color: var(--label-tertiary); margin-top: 1px;">{root.label}</span>
                                        </button>
                                    {/each}
                                    <div style="height: 1px; background: var(--border-subtle); margin: 4px 0;"></div>
                                {/if}
                                <button class="move-option" on:click={async () => { showRootPicker = false; await browseDataRoot(); }}>
                                    <span style="font-size: 11px; color: var(--label-primary); display: flex; align-items: center; gap: 6px;">
                                        <FolderOpen size={11} /> Browse for custom folder…
                                    </span>
                                </button>
                            </div>
                        {/if}
                    </div>
                    {#if $storageConfig?.data_root}
                        <button class="btn-secondary" on:click={resetDataRoot} disabled={storageSaving}>
                            <RotateCcw size={12} /> Reset to default
                        </button>
                    {/if}
                </div>
            </div>

            {#if $storageConfig}
                <div class="settings-row" style="gap: 24px; border-bottom: none;">
                    <div class="flex-1 min-w-0">
                        <p class="path-label">Downloads</p>
                        <p class="path-text" title={$storageConfig.effective_download_path}>{$storageConfig.effective_download_path}</p>
                    </div>
                    <div style="width: 1px; align-self: stretch; background: var(--border-subtle); flex-shrink: 0;"></div>
                    <div class="flex-1 min-w-0">
                        <p class="path-label">Library</p>
                        <p class="path-text" title={$storageConfig.effective_library_path}>{$storageConfig.effective_library_path}</p>
                    </div>
                </div>
            {/if}
        </div>
        <p class="hint-text">Changes apply to new operations only — existing files stay where they are.</p>

        <!-- Known library locations -->
        <div class="settings-card" style="margin-top: 12px;">
            <div class="settings-row" style="{knownLibraryLocations.length === 0 ? 'border-bottom: none;' : ''}">
                <div class="settings-row-label">
                    <span class="row-title">Library locations</span>
                    <span class="row-desc">All ScrapStation library folders the app manages. Games can be moved between them.</span>
                </div>
                <button class="btn-secondary" on:click={handleAddLibraryLocation} disabled={addingLocation}>
                    <Plus size={12} /> Add folder
                </button>
            </div>
            {#each knownLibraryLocations as loc, i}
                <div class="settings-row" style="{i === knownLibraryLocations.length - 1 ? 'border-bottom: none;' : ''}">
                    <div class="settings-row-label" style="gap: 6px;">
                        <span class="row-title flex items-center gap-2">
                            {loc.label}
                            {#if loc.is_current}
                                <span style="font-size: 9px; font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase;
                                             padding: 2px 6px; border-radius: 4px; background: rgba(10,132,255,0.15); color: #0a84ff;">
                                    Active
                                </span>
                            {:else if !loc.removable}
                                <span style="font-size: 9px; font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase;
                                             padding: 2px 6px; border-radius: 4px; background: rgba(255,255,255,0.06); color: var(--label-quaternary);">
                                    Auto
                                </span>
                            {/if}
                        </span>
                        <span class="path-text" title={loc.path}>{loc.path}</span>
                    </div>
                    {#if loc.removable}
                        <button class="btn-icon-sm"
                                on:click={() => handleRemoveLibraryLocation(loc.path)}
                                title="Remove from known locations (does not delete files)">
                            <Trash2 size={12} />
                        </button>
                    {/if}
                </div>
            {/each}
        </div>
        {#if locationError}
            <p style="font-size: 11px; color: #ff453a; margin: 6px 0 0;">{locationError}</p>
        {/if}

        {#if storageRootChanged}
            <div class="storage-warning">
                <AlertCircle size={13} style="flex-shrink: 0; margin-top: 1px;" />
                <div>
                    <p style="font-size: 12px; font-weight: 500; margin: 0 0 3px;">Your existing library still points to the previous location</p>
                    <p style="font-size: 11px; margin: 0; opacity: 0.8;">Games already in your library are still at their original location and will continue to work. New installs will go to the new path.</p>
                </div>
                <button on:click={() => storageRootChanged = false}
                        style="flex-shrink: 0; background: none; border: none; cursor: pointer; color: inherit; opacity: 0.6; padding: 0; line-height: 1;">✕</button>
            </div>
        {/if}
    </section>

    <div class="subsection-divider"></div>

    <!-- ══ Sources ══════════════════════════════════════════════════════════ -->
    <section id="section-sources">
        <div class="section-header">
            <FolderOpen size={14} style="color: #32d74b; flex-shrink: 0; margin-top: 2px;" />
            <div>
                <h2 class="section-title">Sources</h2>
                <p class="section-desc">Drop <code>.yaml</code> config files here to install sources. The app reloads them automatically.</p>
            </div>
        </div>

        <div class="settings-card">
            <div class="settings-row" style="border-bottom: none;">
                <div class="settings-row-label">
                    <span class="row-title">Sources folder</span>
                    <span class="row-desc path-text" title={sourcesFolderPath}>{sourcesFolderPath || 'Loading…'}</span>
                </div>
                <button class="btn-secondary" on:click={openSourcesFolder}>
                    <ExternalLink size={12} /> Open folder
                </button>
            </div>
        </div>
    </section>

    <div class="subsection-divider"></div>

    <!-- ══ Recovery ══════════════════════════════════════════════════════════ -->
    <section id="section-recovery">
        <div class="section-header">
            <Wrench size={14} style="color: #ff9f0a; flex-shrink: 0; margin-top: 2px;" />
            <div>
                <h2 class="section-title">Recovery</h2>
                <p class="section-desc">Tools to fix your library or downloads if they appear broken after an update or storage move.</p>
            </div>
        </div>

        <div class="flex flex-col gap-3">
            <div class="settings-card">
                <div class="settings-row" style="border-bottom: {migrationResult ? '1px solid var(--border-subtle)' : 'none'};">
                    <div class="settings-row-label">
                        <span class="row-title">Migrate AppData folder</span>
                        <span class="row-desc">Moves files from the old <code>com.scrapstation.app</code> folder to the current one.</span>
                    </div>
                    <button class="btn-secondary" on:click={runMigration} disabled={migrationRunning} style="flex-shrink: 0;">
                        {#if migrationRunning}
                            <div class="spin" style="width: 12px; height: 12px; border-radius: 50%; border: 1.5px solid rgba(255,255,255,0.15); border-top-color: rgba(255,255,255,0.6);"></div>
                        {:else}
                            <Wrench size={12} />
                        {/if}
                        Run
                    </button>
                </div>
                {#if migrationResult}
                    <div class="result-block">
                        <p class="result-line" style="color: {migrationResult.success ? '#32d74b' : '#ff453a'};">
                            {#if migrationResult.success}<CheckCircle size={12} />{:else}<XCircle size={12} />{/if}
                            {migrationResult.message}
                        </p>
                        {#each migrationResult.details as detail}
                            <p class="result-detail">· {detail}</p>
                        {/each}
                    </div>
                {/if}
            </div>

            <div class="settings-card">
                <div class="settings-row" style="border-bottom: {pathFixResult ? '1px solid var(--border-subtle)' : 'none'};">
                    <div class="settings-row-label">
                        <span class="row-title">Fix broken paths</span>
                        <span class="row-desc">Converts legacy relative paths to absolute in the library database so games are always found regardless of data root.</span>
                    </div>
                    <button class="btn-secondary" on:click={runFixPaths} disabled={pathFixRunning} style="flex-shrink: 0;">
                        {#if pathFixRunning}
                            <div class="spin" style="width: 12px; height: 12px; border-radius: 50%; border: 1.5px solid rgba(255,255,255,0.15); border-top-color: rgba(255,255,255,0.6);"></div>
                        {:else}
                            <Wrench size={12} />
                        {/if}
                        Run
                    </button>
                </div>
                {#if pathFixResult}
                    <div class="result-block">
                        <p class="result-line" style="color: {pathFixResult.success ? '#32d74b' : '#ff453a'};">
                            {#if pathFixResult.success}<CheckCircle size={12} />{:else}<XCircle size={12} />{/if}
                            {pathFixResult.message}
                        </p>
                        {#each pathFixResult.details as detail}
                            <p class="result-detail">· {detail}</p>
                        {/each}
                    </div>
                {/if}
            </div>
        </div>
    </section>

    <!-- ══ Source settings ══════════════════════════════════════════════════ -->
    {#if loading || sourceSettings.length > 0}
        <div class="section-divider"></div>
    {/if}

    {#if loading}
        <section id="section-source-settings">
            <div class="section-header">
                <div class="spin" style="width: 14px; height: 14px; border-radius: 50%; border: 1.5px solid rgba(255,255,255,0.1); border-top-color: rgba(255,255,255,0.5); flex-shrink: 0; margin-top: 2px;"></div>
                <div>
                    <h2 class="section-title">Source Settings</h2>
                    <p class="section-desc">Loading per-source configuration…</p>
                </div>
            </div>
        </section>
    {:else if sourceSettings.length > 0}
        <section id="section-source-settings">
            <div class="section-header">
                <Settings size={14} style="color: #5e5ce6; flex-shrink: 0; margin-top: 2px;" />
                <div>
                    <h2 class="section-title">Source Settings</h2>
                    <p class="section-desc">Per-source configuration — each source defines its own fields in its YAML file.</p>
                </div>
            </div>

            <div class="flex flex-col gap-3">
                {#each sourceSettings as sourceConfig}
                    {@const source = getSource(sourceConfig.sourceId)}
                    {#if source}
                        {@const IconComponent = getIconComponent(source.icon)}
                        <div id="source-{sourceConfig.sourceId}" class="settings-card" style="
                            border-color: {sourceConfig.hasChanges
                                ? 'rgba(255,159,10,0.35)'
                                : hasConfiguredSettings(sourceConfig)
                                    ? 'rgba(50,215,75,0.25)'
                                    : 'rgba(255,255,255,0.08)'};
                        ">
                            <button class="settings-row w-full text-left"
                                    style="border-bottom: {sourceConfig.expanded ? '1px solid var(--border-subtle)' : 'none'}; cursor: pointer;"
                                    on:click={() => toggleExpanded(sourceConfig.sourceId)}>
                                <div class="flex items-center gap-3">
                                    <div style="width: 32px; height: 32px; border-radius: 9px; display: flex; align-items: center; justify-content: center; background: {source.color}12; border: 1px solid {source.color}28; flex-shrink: 0;">
                                        <svelte:component this={IconComponent} size={15} style="color: {source.color};" />
                                    </div>
                                    <div>
                                        <p style="font-size: 13px; font-weight: 500; color: var(--label-primary);">{source.name}</p>
                                        <div class="flex items-center gap-2 mt-0.5">
                                            {#if hasConfiguredSettings(sourceConfig)}
                                                <span style="font-size: 11px; color: #32d74b; display: flex; align-items: center; gap: 4px;">
                                                    <Check size={10} /> Configured
                                                </span>
                                            {:else}
                                                <span style="font-size: 11px; color: var(--label-quaternary);">Not configured</span>
                                            {/if}
                                            {#if sourceConfig.hasChanges}
                                                <span style="font-size: 11px; color: #ff9f0a;">· Unsaved changes</span>
                                            {/if}
                                        </div>
                                    </div>
                                </div>
                                <div style="color: var(--label-tertiary);">
                                    {#if sourceConfig.expanded}<ChevronDown size={15} />{:else}<ChevronRight size={15} />{/if}
                                </div>
                            </button>

                            {#if sourceConfig.expanded}
                                <div style="padding: 16px;">
                                    {#if sourceConfig.settingSections && sourceConfig.settingSections.length > 0}
                                        <SettingSectionRenderer sourceId={sourceConfig.sourceId} sections={sourceConfig.settingSections} />
                                    {:else}
                                        <div class="flex flex-col gap-3">
                                            {#each sourceConfig.schema as setting}
                                                <LegacySettingField
                                                    {setting}
                                                    value={sourceConfig.editValues[setting.id] || ''}
                                                    showSecret={sourceConfig.showSecrets[setting.id] || false}
                                                    authState={sourceConfig.authState}
                                                    onValueChange={(v) => updateValue(sourceConfig.sourceId, setting.id, v)}
                                                    onToggleSecret={() => toggleSecret(sourceConfig.sourceId, setting.id)}
                                                    onLogout={() => handleLogout(sourceConfig.sourceId)}
                                                    onSsoLogin={(pid) => handleSsoLogin(sourceConfig.sourceId, pid)}
                                                    onRefreshAuthStatus={() => refreshAuthStatus(sourceConfig.sourceId)}
                                                />
                                            {/each}
                                        </div>
                                    {/if}
                                    {#if sourceConfig.hasChanges}
                                        <div class="flex justify-end gap-2 mt-4 pt-4" style="border-top: 1px solid var(--border-subtle);">
                                            <button class="btn-secondary" on:click={() => resetSettings(sourceConfig.sourceId)} disabled={sourceConfig.isSaving}>
                                                <RotateCcw size={12} /> Discard
                                            </button>
                                            <button class="btn-primary" on:click={() => saveSettings(sourceConfig.sourceId)} disabled={sourceConfig.isSaving}>
                                                <Save size={12} /> {sourceConfig.isSaving ? 'Saving…' : 'Save changes'}
                                            </button>
                                        </div>
                                    {/if}
                                </div>
                            {/if}
                        </div>
                    {/if}
                {/each}
            </div>
        </section>
    {/if}


</div>
    </div><!-- /right scroll -->
</div><!-- /flex wrapper -->

<style>
    .settings-card {
        border-radius: 10px;
        border: 1px solid rgba(255,255,255,0.08);
        background: rgba(255,255,255,0.02);
        overflow: hidden;
    }

    .settings-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
        padding: 14px 16px;
        border-bottom: 1px solid var(--border-subtle);
    }

    .settings-row-label {
        display: flex;
        flex-direction: column;
        gap: 3px;
        min-width: 0;
        flex: 1;
    }

    .row-title {
        font-size: 13px;
        font-weight: 500;
        color: var(--label-primary);
    }

    .row-desc {
        font-size: 12px;
        color: var(--label-tertiary);
    }

    .path-text {
        font-family: ui-monospace, 'Cascadia Code', monospace;
        font-size: 11px;
        color: var(--label-quaternary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        display: block;
    }

    code {
        font-family: ui-monospace, 'Cascadia Code', monospace;
        font-size: 11px;
        background: rgba(255,255,255,0.07);
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 4px;
        padding: 1px 5px;
        color: var(--label-secondary);
    }

    .btn-secondary {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 6px 12px;
        font-size: 12px;
        font-weight: 500;
        color: var(--label-secondary);
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 7px;
        cursor: pointer;
        transition: background 0.15s, color 0.15s;
        white-space: nowrap;
    }
    .btn-secondary:hover:not(:disabled) {
        background: rgba(255,255,255,0.09);
        color: var(--label-primary);
    }
    .btn-secondary:disabled {
        opacity: 0.4;
        cursor: not-allowed;
    }

    .btn-primary {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 6px 14px;
        font-size: 12px;
        font-weight: 600;
        color: #000;
        background: #fff;
        border: 1px solid transparent;
        border-radius: 7px;
        cursor: pointer;
        transition: background 0.15s;
        white-space: nowrap;
    }
    .btn-primary:hover:not(:disabled) { background: #e8e8e8; }
    .btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

    .btn-icon-sm {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        border-radius: 6px;
        border: 1px solid rgba(255,255,255,0.08);
        background: transparent;
        color: var(--label-tertiary);
        cursor: pointer;
        flex-shrink: 0;
        transition: background 0.12s, color 0.12s;
    }
    .btn-icon-sm:hover {
        background: rgba(255,69,58,0.12);
        color: #ff453a;
        border-color: rgba(255,69,58,0.25);
    }

    /* ── Section layout ──────────────────────────────────────────────────── */
    .section-header {
        display: flex;
        align-items: flex-start;
        gap: 10px;
        margin-bottom: 16px;
    }

    .section-title {
        font-size: 14px;
        font-weight: 600;
        color: var(--label-primary);
        letter-spacing: -0.01em;
        margin: 0 0 3px;
        line-height: 1.2;
    }

    .section-desc {
        font-size: 12px;
        color: var(--label-tertiary);
        margin: 0;
        line-height: 1.5;
    }

    .hint-text {
        font-size: 11px;
        color: var(--label-quaternary);
        margin: 8px 2px 0;
    }

    .storage-warning {
        display: flex;
        align-items: flex-start;
        gap: 10px;
        margin-top: 12px;
        padding: 12px 14px;
        border-radius: 9px;
        background: rgba(255, 159, 10, 0.08);
        border: 1px solid rgba(255, 159, 10, 0.25);
        color: #ff9f0a;
    }

    .path-label {
        font-size: 10px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.07em;
        color: var(--label-quaternary);
        margin: 0 0 4px;
    }

    .section-divider {
        margin: 32px -40px;
        height: 1px;
        background: var(--border-subtle);
    }

    .subsection-divider {
        margin: 28px 0;
        height: 1px;
        background: var(--border-subtle);
    }

    /* ── Result blocks ───────────────────────────────────────────────────── */
    .result-block {
        padding: 12px 16px;
        display: flex;
        flex-direction: column;
        gap: 4px;
        background: rgba(255,255,255,0.015);
    }

    .result-line {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 12px;
        font-weight: 500;
        margin: 0;
    }

    .result-detail {
        font-size: 11px;
        color: var(--label-quaternary);
        padding-left: 18px;
        margin: 0;
    }

    .spin { animation: spin 0.8s linear infinite; }
    @keyframes spin { to { transform: rotate(360deg); } }

    section h2 { margin: 0; }

    .nav-label {
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--label-quaternary);
        padding: 4px 10px 2px;
        margin-top: 4px;
    }

    .nav-item {
        display: flex;
        align-items: center;
        gap: 9px;
        width: 100%;
        text-align: left;
        padding: 7px 10px;
        border-radius: 7px;
        font-size: 13px;
        color: var(--label-secondary);
        background: transparent;
        border: none;
        cursor: pointer;
        transition: background 0.12s, color 0.12s;
    }
    .nav-item:hover { background: rgba(255,255,255,0.06); color: var(--label-primary); }
    .nav-active {
        background: rgba(255,255,255,0.07) !important;
        color: var(--label-primary) !important;
        font-weight: 500;
        box-shadow: inset 2px 0 0 rgba(255,255,255,0.25);
    }

    /* ── Root picker dropdown ────────────────────────────────────────────── */
    .move-dropdown {
        position: absolute;
        top: calc(100% + 6px);
        right: 0;
        z-index: 200;
        background: #1c1c1e;
        border: 1px solid rgba(255,255,255,0.12);
        border-radius: 10px;
        padding: 6px;
        min-width: 200px;
        box-shadow: 0 8px 32px rgba(0,0,0,0.5);
    }

    .move-dropdown-header {
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.07em;
        text-transform: uppercase;
        color: var(--label-quaternary);
        padding: 4px 8px 6px;
    }

    .move-option {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        width: 100%;
        text-align: left;
        padding: 8px 10px;
        border-radius: 7px;
        border: none;
        background: transparent;
        cursor: pointer;
        transition: background 0.12s;
        color: var(--label-primary);
    }
    .move-option:hover { background: rgba(255,255,255,0.07); }
</style>

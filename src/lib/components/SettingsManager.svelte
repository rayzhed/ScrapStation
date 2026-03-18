<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { listen, type UnlistenFn } from '@tauri-apps/api/event';
    import { sources } from '$lib/stores/sources';
    import { X, Settings, ChevronDown, ChevronRight, Save, RotateCcw, AlertCircle, Check, FolderOpen, ExternalLink } from 'lucide-svelte';
    import * as LucideIcons from 'lucide-svelte';
    import { onMount, onDestroy } from 'svelte';
    import { animate } from 'motion';
    import SettingSectionRenderer from './SettingSectionRenderer.svelte';
    import LegacySettingField from './settings/LegacySettingField.svelte';

    export let isOpen = false;
    export let onClose: () => void;

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

    // Modal entrance: scale up with spring + fade
    function modalIn(node: HTMLElement) {
        node.style.opacity = '0';
        animate(
            node,
            { opacity: [0, 1], scale: [0.94, 1], y: [10, 0] },
            { duration: 0.32, easing: [0.22, 1, 0.36, 1] }
        );
    }
    let hasLoaded = false;
    let justLoaded = false;
    let ssoAuthUnlisten: UnlistenFn | null = null;

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
    });

    $: if (isOpen) {
        if (!hasLoaded && !loading) {
            loadAllSettings();
        } else if (hasLoaded && !justLoaded) {
            refreshAllAuthStatus();
        }
        if (justLoaded) justLoaded = false;
    }

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
        justLoaded = true;
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

    function handleBackdropClick(e: MouseEvent) {
        if (e.target === e.currentTarget) onClose();
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') onClose();
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

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="fixed inset-0 z-[100] flex items-center justify-center bg-black/80"
        style="top: 32px;"
        on:click={handleBackdropClick}
    >
        <div class="relative w-full max-w-xl max-h-[80vh] mx-4 sc-card overflow-hidden flex flex-col"
             style="box-shadow: 0 0 0 1px rgba(255,255,255,0.08), 0 8px 32px rgba(0,0,0,0.6), 0 24px 80px rgba(0,0,0,0.5);"
             use:modalIn>
            <!-- Header -->
            <div class="flex items-center justify-between px-5 py-3.5 border-b border-white/8 flex-shrink-0">
                <div class="flex items-center gap-2.5">
                    <Settings size={15} class="text-white/50" />
                    <div>
                        <h2 class="text-xs font-bold text-white tracking-wider">SETTINGS</h2>
                        <p class="text-[11px]" style="color: var(--label-tertiary);">Configure sources and preferences</p>
                    </div>
                </div>
                <button on:click={onClose} class="p-1.5 rounded-subtle hover:bg-white/8 transition-colors">
                    <X size={14} class="text-white/40" />
                </button>
            </div>

            <!-- Content -->
            <div class="flex-1 overflow-y-auto p-4">
                <!-- Sources Folder Card -->
                <div class="mb-4 p-3 border border-white/8 rounded-subtle">
                    <div class="flex items-center justify-between">
                        <div class="flex items-center gap-2.5">
                            <div class="w-7 h-7 rounded-subtle flex items-center justify-center border border-white/10 bg-white/5">
                                <FolderOpen size={13} class="text-white/60" />
                            </div>
                            <div>
                                <p class="text-xs font-medium text-white/80">Sources Folder</p>
                                <p class="text-[11px] mt-0.5 font-mono truncate max-w-[280px]"
                                   style="color: var(--label-tertiary);"
                                   title={sourcesFolderPath}>{sourcesFolderPath || 'Loading...'}</p>
                            </div>
                        </div>
                        <button
                            on:click={openSourcesFolder}
                            class="flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-subtle border border-white/10 hover:bg-white/8 transition-colors"
                            style="color: var(--label-secondary);"
                            title="Open in file explorer"
                        >
                            <ExternalLink size={11} />
                            Open
                        </button>
                    </div>
                    <p class="text-[11px] mt-2.5" style="color: var(--label-quaternary);">
                        Drop your <span class="font-mono text-white/40">.yaml</span> source configs here. The app reloads them automatically.
                    </p>
                </div>

                {#if loading}
                    <div class="flex items-center justify-center py-10">
                        <div class="w-6 h-6 rounded-full animate-spin"
                             style="border: 1.5px solid rgba(255,255,255,0.1); border-top-color: rgba(255,255,255,0.5);"></div>
                    </div>
                {:else if sourceSettings.length === 0}
                    <div class="flex flex-col items-center justify-center py-10 text-center">
                        <Settings size={32} class="mb-3" style="color: var(--label-quaternary);" />
                        <p class="text-[12px]" style="color: var(--label-tertiary);">No configurable sources</p>
                        <p class="text-[11px] mt-1" style="color: var(--label-tertiary);">Sources can define settings in their YAML configuration</p>
                    </div>
                {:else}
                    <div class="space-y-2">
                        {#each sourceSettings as sourceConfig}
                            {@const source = getSource(sourceConfig.sourceId)}
                            {#if source}
                                {@const IconComponent = getIconComponent(source.icon)}
                                <div class="border rounded-subtle overflow-hidden transition-colors
                                    {sourceConfig.hasChanges ? 'border-amber-400/25' : hasConfiguredSettings(sourceConfig) ? 'border-green-400/15' : 'border-white/8'}">

                                    <!-- Source Header -->
                                    <button
                                        on:click={() => toggleExpanded(sourceConfig.sourceId)}
                                        class="w-full flex items-center justify-between p-3 hover:bg-white/3 transition-colors"
                                    >
                                        <div class="flex items-center gap-2.5">
                                            <div class="w-7 h-7 rounded-subtle flex items-center justify-center border"
                                                style="border-color: {source.color}25; background: {source.color}0a;">
                                                <svelte:component this={IconComponent} size={14} style="color: {source.color};" />
                                            </div>
                                            <div class="text-left">
                                                <span class="text-xs font-medium text-white/80">{source.name}</span>
                                                <div class="flex items-center gap-1.5 mt-0.5">
                                                    {#if hasConfiguredSettings(sourceConfig)}
                                                        <span class="flex items-center gap-0.5 text-[11px]" style="color: #32d74b;">
                                                            <Check size={10} /> Configured
                                                        </span>
                                                    {:else}
                                                        <span class="text-[11px]" style="color: var(--label-tertiary);">Not configured</span>
                                                    {/if}
                                                    {#if sourceConfig.hasChanges}
                                                        <span class="text-[11px]" style="color: #ff9f0a;">· Unsaved changes</span>
                                                    {/if}
                                                </div>
                                            </div>
                                        </div>
                                        <div style="color: var(--label-tertiary);">
                                            {#if sourceConfig.expanded}<ChevronDown size={16} />{:else}<ChevronRight size={16} />{/if}
                                        </div>
                                    </button>

                                    <!-- Settings Fields -->
                                    {#if sourceConfig.expanded}
                                        <div class="px-3 pb-3 space-y-3 border-t border-white/5">
                                            {#if sourceConfig.settingSections && sourceConfig.settingSections.length > 0}
                                                <div class="pt-3">
                                                    <SettingSectionRenderer sourceId={sourceConfig.sourceId} sections={sourceConfig.settingSections} />
                                                </div>
                                            {:else}
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
                                            {/if}

                                            {#if sourceConfig.hasChanges}
                                                <div class="flex justify-end gap-2 pt-3 border-t border-white/5">
                                                    <button
                                                        on:click={() => resetSettings(sourceConfig.sourceId)}
                                                        class="btn-secondary flex items-center gap-1.5 text-[11px]"
                                                        style="padding: 5px 12px;"
                                                        disabled={sourceConfig.isSaving}
                                                    >
                                                        <RotateCcw size={12} /> Reset
                                                    </button>
                                                    <button
                                                        on:click={() => saveSettings(sourceConfig.sourceId)}
                                                        class="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-subtle bg-white text-black hover:bg-gray-200 transition-colors disabled:opacity-50"
                                                        disabled={sourceConfig.isSaving}
                                                    >
                                                        <Save size={12} /> {sourceConfig.isSaving ? 'Saving...' : 'Save'}
                                                    </button>
                                                </div>
                                            {/if}
                                        </div>
                                    {/if}
                                </div>
                            {/if}
                        {/each}
                    </div>
                {/if}

                {#if !loading && sourceSettings.length > 0}
                    <div class="mt-4 p-3 border border-white/5 rounded-subtle">
                        <div class="flex items-start gap-2">
                            <AlertCircle size={13} class="mt-0.5 flex-shrink-0" style="color: var(--label-tertiary);" />
                            <div class="text-[11px]" style="color: var(--label-tertiary);">
                                <p>Settings are defined by each source in their configuration file.</p>
                                <p class="mt-0.5">Changes are saved per-source and persist between sessions.</p>
                            </div>
                        </div>
                    </div>
                {/if}

            </div>
        </div>
    </div>
{/if}

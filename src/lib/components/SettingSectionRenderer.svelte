<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { listen, type UnlistenFn } from '@tauri-apps/api/event';
    import { onMount, onDestroy } from 'svelte';
    import * as LucideIcons from 'lucide-svelte';
    import { Loader2, Check, AlertCircle } from 'lucide-svelte';

    export let sourceId: string;
    export let sections: SettingSection[] = [];

    interface VisibilityCondition {
        key: string;
        exists?: boolean;
        equals?: any;
        contains?: string;
    }

    interface ActionConfig {
        url?: string;
        wait_for_cookie?: string;
        close_on_domains?: string[];
        store_cookies_as?: string;
        keys?: string[];
        clear_webview?: boolean;
        key?: string;
        value?: any;
        link?: string;
    }

    interface ButtonConfig {
        label: string;
        icon?: string;
        color?: string;
        variant?: string;
        action: string;
        action_config?: ActionConfig;
    }

    interface SettingComponent {
        type: string;
        show_when?: VisibilityCondition;
        hide_when?: VisibilityCondition;
        // Button group
        buttons?: ButtonConfig[];
        // Button (flattened)
        label?: string;
        icon?: string;
        color?: string;
        variant?: string;
        action?: string;
        action_config?: ActionConfig;
        // Toggle/Input/Select
        id?: string;
        description?: string;
        placeholder?: string;
        secret?: boolean;
        default?: any;
        store_as?: string;
        options?: { value: string; label: string }[];
        // Status card
        text?: string;
        // Text
        content?: string;
    }

    interface SettingSection {
        id: string;
        title: string;
        icon?: string;
        description?: string;
        components: SettingComponent[];
    }

    interface AuthResult {
        success: boolean;
        cookies?: string;
        username?: string;
        error?: string;
        source_id: string;
    }

    // Storage values for condition checking
    let storageValues: Record<string, any> = {};
    let loadingActions: Set<string> = new Set();
    let actionErrors: Record<string, string> = {};
    let actionSuccess: Record<string, string> = {};
    let authUnlisten: UnlistenFn | null = null;

    function getIconComponent(iconName: string) {
        if (!iconName) return LucideIcons.Settings;
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        // @ts-ignore
        return LucideIcons[pascalCase] || LucideIcons.Settings;
    }

    onMount(async () => {
        await loadStorageValues();
        await setupAuthListener();
    });

    onDestroy(() => {
        if (authUnlisten) authUnlisten();
    });

    async function setupAuthListener() {
        authUnlisten = await listen<AuthResult>('auth-complete', async (event) => {
            const result = event.payload;
            if (result.source_id !== sourceId) return;

            loadingActions = new Set();

            if (result.success) {
                actionSuccess['auth'] = 'Logged in successfully!';
                setTimeout(() => {
                    actionSuccess = {};
                }, 3000);
            } else if (result.error && result.error !== 'Authentication cancelled') {
                actionErrors['auth'] = result.error;
            }

            // Refresh storage values
            await loadStorageValues();
        });
    }

    async function loadStorageValues() {
        try {
            const values = await invoke<Record<string, any>>('get_source_settings_values', { sourceId });
            storageValues = values;
        } catch (e) {
        }
    }

    function checkCondition(condition: VisibilityCondition | undefined): boolean {
        if (!condition) return true;

        const value = storageValues[condition.key];
        const exists = value !== undefined && value !== null && value !== '';

        if (condition.exists !== undefined) {
            return condition.exists ? exists : !exists;
        }

        if (condition.equals !== undefined) {
            return value === condition.equals;
        }

        if (condition.contains !== undefined && typeof value === 'string') {
            return value.includes(condition.contains);
        }

        return true;
    }

    function shouldShowComponent(component: SettingComponent): boolean {
        if (component.hide_when && checkCondition(component.hide_when)) {
            return false;
        }
        if (component.show_when && !checkCondition(component.show_when)) {
            return false;
        }
        return true;
    }

    async function executeAction(action: string, config: ActionConfig | undefined, buttonId: string) {
        loadingActions.add(buttonId);
        loadingActions = loadingActions;
        actionErrors = {};

        try {
            await invoke('execute_action', {
                sourceId,
                action,
                config: config || {}
            });

            // For non-webview actions, show success
            if (action !== 'open_webview') {
                await loadStorageValues();
            }
        } catch (e: any) {
            actionErrors[buttonId] = e.toString();
        } finally {
            if (action !== 'open_webview') {
                loadingActions.delete(buttonId);
                loadingActions = loadingActions;
            }
        }
    }

    async function handleToggleChange(storeAs: string, value: boolean) {
        try {
            await invoke('set_source_setting', {
                sourceId,
                settingId: storeAs,
                value
            });
            storageValues[storeAs] = value;
            storageValues = storageValues;
        } catch (e) {
        }
    }

    async function handleInputChange(storeAs: string, value: string) {
        try {
            await invoke('set_source_setting', {
                sourceId,
                settingId: storeAs,
                value
            });
            storageValues[storeAs] = value;
        } catch (e) {
        }
    }

    function getVariantClasses(variant: string | undefined): string {
        switch (variant) {
            case 'success': return 'bg-green-500/10 border-green-500/30 text-green-400';
            case 'warning': return 'bg-yellow-500/10 border-yellow-500/30 text-yellow-400';
            case 'error': return 'bg-red-500/10 border-red-500/30 text-red-400';
            case 'danger': return 'bg-red-500/20 hover:bg-red-500/30 border-red-500/30 text-red-400';
            case 'primary': return 'bg-accent-primary/20 hover:bg-accent-primary/30 border-accent-primary/30';
            case 'secondary': return 'bg-white/5 hover:bg-white/10 border-white/10';
            default: return 'bg-white/5 hover:bg-white/10 border-white/10';
        }
    }
</script>

{#each sections as section}
    <div class="mb-6">
        <!-- Section header -->
        <div class="flex items-center gap-2 mb-3">
            <svelte:component this={getIconComponent(section.icon || 'settings')} size={18} class="text-white/60" />
            <h4 class="text-sm font-medium text-white">{section.title}</h4>
        </div>

        {#if section.description}
            <p class="text-xs text-white/40 mb-3">{section.description}</p>
        {/if}

        <!-- Components -->
        <div class="space-y-3">
            {#each section.components as component}
                {#if shouldShowComponent(component)}
                    <!-- Button Group -->
                    {#if component.type === 'button_group' && component.buttons}
                        <div class="flex flex-wrap gap-2">
                            {#each component.buttons as button, idx}
                                {@const buttonId = `${section.id}-btn-${idx}`}
                                <button
                                    on:click={() => executeAction(button.action, button.action_config, buttonId)}
                                    disabled={loadingActions.has(buttonId)}
                                    class="flex items-center gap-2 px-4 py-2.5 rounded-lg border transition-all text-sm font-medium disabled:opacity-50 {button.color ? '' : 'bg-white/5 hover:bg-white/10 border-white/10'}"
                                    style={button.color ? `border-color: ${button.color}40; background: ${button.color}15;` : ''}
                                >
                                    {#if loadingActions.has(buttonId)}
                                        <Loader2 size={16} class="animate-spin" />
                                    {:else if button.icon}
                                        <svelte:component this={getIconComponent(button.icon)} size={16} />
                                    {/if}
                                    <span>{button.label}</span>
                                </button>
                            {/each}
                        </div>

                    <!-- Single Button -->
                    {:else if component.type === 'button'}
                        {@const buttonId = `${section.id}-${component.label}`}
                        <button
                            on:click={() => executeAction(component.action || '', component.action_config, buttonId)}
                            disabled={loadingActions.has(buttonId)}
                            class="flex items-center gap-2 px-4 py-2 rounded-lg border transition-all text-sm font-medium disabled:opacity-50 {getVariantClasses(component.variant)}"
                        >
                            {#if loadingActions.has(buttonId)}
                                <Loader2 size={16} class="animate-spin" />
                            {:else if component.icon}
                                <svelte:component this={getIconComponent(component.icon)} size={16} />
                            {/if}
                            <span>{component.label}</span>
                        </button>

                    <!-- Toggle -->
                    {:else if component.type === 'toggle' && component.store_as}
                        {@const currentValue = storageValues[component.store_as] ?? component.default ?? false}
                        <div class="flex items-center justify-between py-2">
                            <div>
                                <div class="text-sm text-white">{component.label}</div>
                                {#if component.description}
                                    <div class="text-xs text-white/40">{component.description}</div>
                                {/if}
                            </div>
                            <button
                                on:click={() => handleToggleChange(component.store_as || '', !currentValue)}
                                class="relative w-10 h-5 rounded-full transition-colors {currentValue ? 'bg-accent-primary' : 'bg-white/20'}"
                                aria-label="Toggle {component.label}"
                            >
                                <span
                                    class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {currentValue ? 'translate-x-5' : ''}"
                                ></span>
                            </button>
                        </div>

                    <!-- Input -->
                    {:else if component.type === 'input' && component.store_as}
                        {@const currentValue = storageValues[component.store_as] ?? component.default ?? ''}
                        {@const inputId = `input-${component.store_as}`}
                        <div class="space-y-1">
                            <label for={inputId} class="text-sm text-white">{component.label}</label>
                            {#if component.description}
                                <p class="text-xs text-white/40">{component.description}</p>
                            {/if}
                            <input
                                id={inputId}
                                type={component.secret ? 'password' : 'text'}
                                value={currentValue}
                                placeholder={component.placeholder}
                                on:change={(e) => handleInputChange(component.store_as || '', e.currentTarget.value)}
                                class="w-full px-3 py-2 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-white/30 focus:border-accent-primary/50 focus:outline-none"
                            />
                        </div>

                    <!-- Select -->
                    {:else if component.type === 'select' && component.store_as && component.options}
                        {@const currentValue = storageValues[component.store_as] ?? component.default ?? ''}
                        {@const selectId = `select-${component.store_as}`}
                        <div class="space-y-1">
                            <label for={selectId} class="text-sm text-white">{component.label}</label>
                            {#if component.description}
                                <p class="text-xs text-white/40">{component.description}</p>
                            {/if}
                            <select
                                id={selectId}
                                value={currentValue}
                                on:change={(e) => handleInputChange(component.store_as || '', e.currentTarget.value)}
                                class="w-full px-3 py-2 bg-white/5 border border-white/10 rounded-lg text-sm text-white focus:border-accent-primary/50 focus:outline-none"
                            >
                                {#each component.options as option}
                                    <option value={option.value}>{option.label}</option>
                                {/each}
                            </select>
                        </div>

                    <!-- Status Card -->
                    {:else if component.type === 'status_card'}
                        <div class="flex items-center gap-3 px-4 py-3 rounded-lg border {getVariantClasses(component.variant)}">
                            {#if component.icon}
                                <svelte:component this={getIconComponent(component.icon)} size={18} />
                            {/if}
                            <span class="text-sm">{component.text}</span>
                        </div>

                    <!-- Text -->
                    {:else if component.type === 'text'}
                        <p class="text-sm {component.variant === 'muted' ? 'text-white/40' : component.variant === 'warning' ? 'text-yellow-400' : component.variant === 'error' ? 'text-red-400' : 'text-white/60'}">
                            {component.content}
                        </p>

                    <!-- Divider -->
                    {:else if component.type === 'divider'}
                        <hr class="border-white/10" />
                    {/if}
                {/if}
            {/each}
        </div>

        <!-- Action errors -->
        {#if Object.keys(actionErrors).length > 0}
            <div class="mt-3 flex items-center gap-2 text-red-400 text-xs">
                <AlertCircle size={14} />
                <span>{Object.values(actionErrors)[0]}</span>
            </div>
        {/if}

        <!-- Action success -->
        {#if Object.keys(actionSuccess).length > 0}
            <div class="mt-3 flex items-center gap-2 text-green-400 text-xs">
                <Check size={14} />
                <span>{Object.values(actionSuccess)[0]}</span>
            </div>
        {/if}
    </div>
{/each}

<script lang="ts">
    import { User, LogOut, Loader2, RefreshCw } from 'lucide-svelte';
    import * as LucideIcons from 'lucide-svelte';

    export let authState: {
        isLoggedIn: boolean;
        username?: string;
        isLoading: boolean;
        error?: string;
        success?: string;
    };
    export let authConfig: {
        sso?: Array<{
            id: string;
            label: string;
            icon?: string;
            color?: string;
        }>;
    };
    export let onLogout: () => void;
    export let onSsoLogin: (providerId: string) => void;
    export let onRefreshStatus: () => void;

    function getIconComponent(iconName: string) {
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        return (LucideIcons as Record<string, any>)[pascalCase] || LucideIcons.LogIn;
    }
</script>

<div class="rounded-lg border border-white/10 bg-black/20 overflow-hidden">
    {#if authState.isLoggedIn}
        <!-- Logged In State -->
        <div class="p-4">
            <div class="flex items-center gap-3 mb-4">
                <div class="w-12 h-12 rounded-full bg-green-500/20 border border-green-500/30 flex items-center justify-center">
                    <User size={24} class="text-green-400" />
                </div>
                <div>
                    <p class="text-lg font-bold text-green-400">Logged in</p>
                    <p class="text-xs text-white/50">Session cookies saved</p>
                </div>
            </div>

            {#if authState.success}
                <div class="mb-4 p-3 rounded-lg bg-green-500/10 border border-green-500/20">
                    <p class="text-xs text-green-400">{authState.success}</p>
                </div>
            {/if}

            <button
                on:click={onLogout}
                disabled={authState.isLoading}
                class="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg
                    bg-red-500/10 hover:bg-red-500/20 border border-red-500/30
                    text-red-400 text-sm font-medium transition-colors disabled:opacity-50"
            >
                {#if authState.isLoading}
                    <Loader2 size={16} class="animate-spin" />
                    Logging out...
                {:else}
                    <LogOut size={16} />
                    Logout
                {/if}
            </button>
        </div>
    {:else}
        <!-- SSO Login -->
        <div class="p-4 space-y-4">
            {#if authState.error}
                <div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20">
                    <p class="text-xs text-red-400">{authState.error}</p>
                </div>
            {/if}
            {#if authState.success}
                <div class="p-3 rounded-lg bg-green-500/10 border border-green-500/20">
                    <p class="text-xs text-green-400">{authState.success}</p>
                </div>
            {/if}

            {#if authConfig.sso && authConfig.sso.length > 0}
                <div class="space-y-3">
                    <p class="text-xs text-white/50 text-center">Login with</p>
                    <div class="grid gap-2" class:grid-cols-2={authConfig.sso.length > 1}>
                        {#each authConfig.sso as provider}
                            {@const ProviderIcon = getIconComponent(provider.icon || 'log-in')}
                            <button
                                on:click={() => onSsoLogin(provider.id)}
                                disabled={authState.isLoading}
                                class="flex items-center justify-center gap-2 px-3 py-2.5 rounded-lg
                                    border transition-all text-sm font-medium disabled:opacity-50"
                                style="background: {provider.color}15; border-color: {provider.color}40; color: {provider.color};"
                            >
                                {#if authState.isLoading}
                                    <Loader2 size={16} class="animate-spin" />
                                {:else}
                                    <svelte:component this={ProviderIcon} size={16} />
                                {/if}
                                {provider.label}
                            </button>
                        {/each}
                    </div>

                    <button
                        on:click={onRefreshStatus}
                        disabled={authState.isLoading}
                        class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg
                            bg-white/5 hover:bg-white/10 border border-white/10
                            text-white/60 text-xs font-medium transition-colors disabled:opacity-50"
                    >
                        <RefreshCw size={14} />
                        Check login status
                    </button>
                </div>
            {:else}
                <div class="text-center py-4">
                    <p class="text-sm text-white/40">No login method configured for this source</p>
                </div>
            {/if}
        </div>
    {/if}
</div>

<script lang="ts">
    import { Eye, EyeOff } from 'lucide-svelte';
    import AuthWidget from './AuthWidget.svelte';

    export let setting: {
        id: string;
        type: 'text' | 'textarea' | 'toggle' | 'select' | 'number' | 'password' | 'auth';
        label: string;
        description?: string;
        placeholder?: string;
        required?: boolean;
        secret?: boolean;
        options?: { value: string; label: string }[];
        config?: any;
    };
    export let value: any;
    export let showSecret: boolean = false;
    export let authState: any = null;

    export let onValueChange: (value: any) => void;
    export let onToggleSecret: () => void;
    export let onLogout: () => void = () => {};
    export let onSsoLogin: (providerId: string) => void = () => {};
    export let onRefreshAuthStatus: () => void = () => {};
</script>

<div class="pt-4">
    <div class="block mb-2">
        <span class="text-sm font-medium text-white">
            {setting.label}
            {#if setting.required}
                <span class="text-red-400">*</span>
            {/if}
        </span>
        {#if setting.description}
            <span class="block text-xs text-white/50 mt-0.5">{setting.description}</span>
        {/if}
    </div>

    {#if setting.type === 'text'}
        <input
            type="text"
            {value}
            placeholder={setting.placeholder}
            on:input={(e) => onValueChange(e.currentTarget.value)}
            class="w-full px-3 py-2 rounded-lg bg-black/30 border border-white/10 text-sm text-white
                placeholder-white/30 focus:outline-none focus:border-accent-primary/30"
        />

    {:else if setting.type === 'textarea'}
        <div class="relative">
            <textarea
                {value}
                placeholder={setting.placeholder}
                on:input={(e) => onValueChange(e.currentTarget.value)}
                rows="3"
                class="w-full px-3 py-2 pr-10 rounded-lg bg-black/30 border border-white/10 text-sm
                    placeholder-white/30 resize-none focus:outline-none focus:border-accent-primary/30
                    {setting.secret && !showSecret ? 'text-transparent' : 'text-white'}"
                style={setting.secret && !showSecret ? 'text-shadow: 0 0 8px rgba(255,255,255,0.5);' : ''}
            ></textarea>
            {#if setting.secret}
                <button
                    on:click={onToggleSecret}
                    class="absolute right-2 top-2 p-1.5 rounded hover:bg-white/5 transition-colors"
                    title={showSecret ? 'Hide' : 'Show'}
                >
                    {#if showSecret}
                        <EyeOff size={16} class="text-white/40" />
                    {:else}
                        <Eye size={16} class="text-white/40" />
                    {/if}
                </button>
            {/if}
        </div>

    {:else if setting.type === 'toggle'}
        <button
            on:click={() => onValueChange(!value)}
            class="relative w-12 h-6 rounded-full transition-colors
                {value ? 'bg-accent-primary' : 'bg-white/20'}"
            aria-label="Toggle {setting.label}"
        >
            <div
                class="absolute top-1 w-4 h-4 rounded-full bg-white transition-transform
                    {value ? 'translate-x-7' : 'translate-x-1'}"
            ></div>
        </button>

    {:else if setting.type === 'select' && setting.options}
        <select
            {value}
            on:change={(e) => onValueChange(e.currentTarget.value)}
            class="w-full px-3 py-2 rounded-lg bg-black/30 border border-white/10 text-sm text-white
                focus:outline-none focus:border-accent-primary/30"
        >
            {#each setting.options as option}
                <option value={option.value}>{option.label}</option>
            {/each}
        </select>

    {:else if setting.type === 'number'}
        <input
            type="number"
            {value}
            placeholder={setting.placeholder}
            on:input={(e) => onValueChange(parseFloat(e.currentTarget.value) || 0)}
            class="w-full px-3 py-2 rounded-lg bg-black/30 border border-white/10 text-sm text-white
                placeholder-white/30 focus:outline-none focus:border-accent-primary/30"
        />

    {:else if setting.type === 'password'}
        <div class="relative">
            <input
                type={showSecret ? 'text' : 'password'}
                {value}
                placeholder={setting.placeholder}
                on:input={(e) => onValueChange(e.currentTarget.value)}
                class="w-full px-3 py-2 pr-10 rounded-lg bg-black/30 border border-white/10 text-sm text-white
                    placeholder-white/30 focus:outline-none focus:border-accent-primary/30"
            />
            <button
                on:click={onToggleSecret}
                class="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded hover:bg-white/5 transition-colors"
            >
                {#if showSecret}
                    <EyeOff size={16} class="text-white/40" />
                {:else}
                    <Eye size={16} class="text-white/40" />
                {/if}
            </button>
        </div>

    {:else if setting.type === 'auth' && setting.config && authState}
        <AuthWidget
            {authState}
            authConfig={setting.config}
            {onLogout}
            {onSsoLogin}
            onRefreshStatus={onRefreshAuthStatus}
        />
    {/if}
</div>

<script lang="ts">
    import * as LucideIcons from 'lucide-svelte';
    import { onMount } from 'svelte';

    export let data: any;

    let visible = false;

    onMount(() => {
        setTimeout(() => visible = true, 100);
    });

    function getIconComponent(iconName: string) {
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        return (LucideIcons as unknown as Record<string, typeof LucideIcons.Info>)[pascalCase] || LucideIcons.Info;
    }
</script>

<div class="relative mb-8 overflow-hidden rounded-[10px]" style="border: 1px solid var(--border-subtle);">
    {#if data.background_image}
        <div class="absolute inset-0 -z-10">
            <img
                src={data.background_image}
                alt={data.title}
                class="w-full h-full object-cover opacity-20"
            />
            <div class="absolute inset-0" style="background: linear-gradient(to bottom, rgba(0,0,0,0.6), rgba(0,0,0,0.88));"></div>
        </div>
    {:else}
        <div class="absolute inset-0 -z-10" style="background: var(--bg-surface);"></div>
    {/if}

    <div class="relative pt-10 pb-8 px-8">
        <h1 class="text-[28px] font-bold mb-3 leading-tight tracking-[-0.025em]
                   transition-all duration-500 {visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}"
            style="color: var(--label-primary);">
            {data.title}
        </h1>

        {#if data.subtitle}
            <p class="text-[13px] mb-6 leading-relaxed
                      transition-all duration-500 delay-75 {visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}"
               style="color: var(--label-secondary);">
                {data.subtitle}
            </p>
        {/if}

        {#if data.badges && data.badges.length > 0}
            <div class="flex gap-2 flex-wrap transition-all duration-500 delay-150 {visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}">
                {#each data.badges as badge}
                    {@const IconComponent = getIconComponent(badge.icon)}
                    <div class="flex items-center gap-2 px-3 py-1.5 rounded-[6px] transition-colors"
                         style="border: 1px solid var(--border); background: rgba(0,0,0,0.35);">
                        <svelte:component this={IconComponent} size={12} strokeWidth={2} style="color: var(--label-tertiary);" />
                        {#if badge.label}
                            <span class="text-[11px] font-medium uppercase" style="letter-spacing: 0.06em; color: var(--label-tertiary);">{badge.label}</span>
                        {/if}
                        <span class="text-[12px] font-semibold" style="color: var(--label-primary);">{badge.value}</span>
                    </div>
                {/each}
            </div>
        {/if}
    </div>

    <div class="h-px" style="background: var(--border-subtle);"></div>
</div>

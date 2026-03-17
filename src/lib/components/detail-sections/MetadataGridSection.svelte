<script lang="ts">
    import * as LucideIcons from 'lucide-svelte';
    import { onMount } from 'svelte';

    export let title: string;
    export let icon: string;
    export let data: any;

    let visible = false;

    onMount(() => {
        setTimeout(() => visible = true, 250);
    });

    function getIconComponent(iconName: string) {
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        return (LucideIcons as unknown as Record<string, typeof LucideIcons.Info>)[pascalCase] || LucideIcons.Info;
    }

    $: IconComponent = getIconComponent(icon);
</script>

<div class="mb-8 transition-all duration-500 {visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}">
    <!-- Section Header -->
    <div class="flex items-center gap-2 mb-4">
        <svelte:component this={IconComponent} size={13} strokeWidth={2} style="color: var(--label-tertiary);" />
        <h2 class="text-[11px] font-semibold uppercase" style="letter-spacing: 0.07em; color: var(--label-secondary);">{title}</h2>
    </div>

    <!-- Metadata Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
        {#each data.items as item}
            {@const ItemIcon = getIconComponent(item.icon)}

            <div class="p-4 rounded-[8px] transition-colors"
                 style="background: var(--bg-surface); border: 1px solid var(--border-subtle);"
                 on:mouseenter={e => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border)'; }}
                 on:mouseleave={e => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-subtle)'; }}>
                <div class="flex items-start gap-3">
                    <div class="flex-shrink-0 w-8 h-8 flex items-center justify-center rounded-[6px]"
                         style="border: 1px solid var(--border-subtle); background: transparent;">
                        <svelte:component this={ItemIcon} size={13} strokeWidth={2} style="color: var(--label-tertiary);" />
                    </div>

                    <div class="flex-1 min-w-0">
                        <p class="text-[11px] font-medium uppercase mb-1.5"
                           style="letter-spacing: 0.06em; color: var(--label-tertiary);">
                            {item.label}
                        </p>

                        {#if item.render_as === 'code'}
                            <code class="inline-block font-mono text-[12px] px-2.5 py-1 rounded-[6px]"
                                  style="color: var(--label-primary); background: var(--bg-raised); border: 1px solid var(--border);">
                                {item.value}
                            </code>
                        {:else if item.render_as === 'link'}
                            <a
                                href={item.value}
                                target="_blank"
                                class="inline-flex items-center gap-1.5 text-[12px] transition-colors"
                                style="color: var(--label-secondary);"
                                on:mouseenter={e => { (e.currentTarget as HTMLElement).style.color = 'var(--label-primary)'; }}
                                on:mouseleave={e => { (e.currentTarget as HTMLElement).style.color = 'var(--label-secondary)'; }}
                            >
                                <span>View on Steam</span>
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
                                </svg>
                            </a>
                        {:else}
                            <p class="text-[12px] font-medium leading-relaxed" style="color: var(--label-primary);">
                                {item.value}
                            </p>
                        {/if}
                    </div>
                </div>
            </div>
        {/each}
    </div>

    <div class="h-px mt-8" style="background: var(--border-subtle);"></div>
</div>

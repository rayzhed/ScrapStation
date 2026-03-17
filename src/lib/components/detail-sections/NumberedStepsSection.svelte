<script lang="ts">
    import * as LucideIcons from 'lucide-svelte';
    import { onMount } from 'svelte';

    export let title: string;
    export let icon: string;
    export let data: any;

    let visible = false;

    onMount(() => {
        setTimeout(() => visible = true, 350);
    });

    function getIconComponent(iconName: string) {
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        return (LucideIcons as unknown as Record<string, typeof LucideIcons.ListOrdered>)[pascalCase] || LucideIcons.ListOrdered;
    }

    $: IconComponent = getIconComponent(icon);
</script>

<div class="mb-8 transition-all duration-500 {visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}">
    <!-- Section Header -->
    <div class="flex items-center gap-2 mb-4">
        <svelte:component this={IconComponent} size={13} strokeWidth={2} style="color: var(--label-tertiary);" />
        <h2 class="text-[11px] font-semibold uppercase" style="letter-spacing: 0.07em; color: var(--label-secondary);">{title}</h2>
    </div>

    <!-- Steps -->
    <div class="rounded-[8px] overflow-hidden" style="background: var(--bg-surface); border: 1px solid var(--border-subtle);">
        {#each data.steps as step, index}
            <div class="flex gap-4 p-4" style="{index < data.steps.length - 1 ? 'border-bottom: 1px solid var(--border-subtle);' : ''}">
                <div class="flex-shrink-0 w-7 h-7 rounded-[6px] flex items-center justify-center font-bold text-[12px]"
                     style="border: 1px solid var(--border); color: var(--label-tertiary);">
                    {step.step_number}
                </div>
                <p class="text-[12px] leading-relaxed pt-0.5 flex-1" style="color: var(--label-secondary);">{step.instruction}</p>
            </div>
        {/each}
    </div>

    <div class="h-px mt-8" style="background: var(--border-subtle);"></div>
</div>

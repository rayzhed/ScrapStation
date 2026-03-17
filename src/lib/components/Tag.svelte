<script lang="ts">
    import * as LucideIcons from 'lucide-svelte';
    import type { GameTag } from '$lib/types';

    export let tag: GameTag;
    export let size: 'sm' | 'md' | 'lg' = 'sm';

    function getIconComponent(iconName?: string) {
        if (!iconName) return null;
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        // @ts-ignore
        return LucideIcons[pascalCase] || null;
    }

    const IconComponent = getIconComponent(tag.icon);

    const sizeClasses = {
        sm: 'text-[11px] px-1.5 py-0.5',
        md: 'text-[11px] px-2 py-0.5',
        lg: 'text-[12px] px-2.5 py-1',
    };

    const iconSizes = {
        sm: 10,
        md: 11,
        lg: 12,
    };
</script>

<div
    class="inline-flex items-center gap-1 font-medium rounded-[5px] border {sizeClasses[size]}"
    style="color: {tag.color || 'var(--label-tertiary)'}; border-color: {tag.color ? tag.color + '30' : 'var(--border-subtle)'}; background: transparent;"
>
    {#if IconComponent}
        <svelte:component this={IconComponent} size={iconSizes[size]} strokeWidth={2} />
    {/if}
    <span>{tag.label}</span>
</div>

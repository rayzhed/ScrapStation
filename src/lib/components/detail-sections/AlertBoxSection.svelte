<script lang="ts">
    import * as LucideIcons from 'lucide-svelte';
    import { onMount } from 'svelte';

    export let title: string;
    export let icon: string;
    export let data: any;

    let visible = false;

    onMount(() => {
        setTimeout(() => visible = true, 400);
    });

    function getIconComponent(iconName: string) {
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        return (LucideIcons as unknown as Record<string, typeof LucideIcons.AlertTriangle>)[pascalCase] || LucideIcons.AlertTriangle;
    }

    function getAlertBorder(style: string): string {
        switch (style) {
            case 'warning': return 'rgba(255,159,10,0.25)';
            case 'danger':  return 'rgba(255,69,58,0.25)';
            case 'success': return 'rgba(50,215,75,0.25)';
            default:        return 'var(--border)';
        }
    }

    function getAlertBg(style: string): string {
        switch (style) {
            case 'warning': return 'rgba(255,159,10,0.06)';
            case 'danger':  return 'rgba(255,69,58,0.06)';
            case 'success': return 'rgba(50,215,75,0.06)';
            default:        return 'rgba(255,255,255,0.03)';
        }
    }

    function getIconColor(style: string): string {
        switch (style) {
            case 'warning': return '#ff9f0a';
            case 'danger':  return '#ff453a';
            case 'success': return '#32d74b';
            default:        return 'var(--label-secondary)';
        }
    }

    function getBulletColor(style: string): string {
        switch (style) {
            case 'warning': return 'rgba(255,159,10,0.5)';
            case 'danger':  return 'rgba(255,69,58,0.5)';
            case 'success': return 'rgba(50,215,75,0.5)';
            default:        return 'var(--label-tertiary)';
        }
    }

    $: IconComponent = getIconComponent(icon);
</script>

<div class="mb-8 transition-all duration-500 {visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}">
    <div class="rounded-[8px] p-5"
         style="border: 1px solid {getAlertBorder(data.style)}; background: {getAlertBg(data.style)};">
        <div class="flex items-center gap-3 mb-4">
            <div class="w-8 h-8 rounded-[6px] flex items-center justify-center shrink-0"
                 style="border: 1px solid var(--border);">
                <svelte:component this={IconComponent} size={15} strokeWidth={2} style="color: {getIconColor(data.style)};" />
            </div>
            <h2 class="text-[13px] font-semibold" style="color: var(--label-primary);">{title}</h2>
        </div>

        <ul class="space-y-2 pl-11">
            {#each data.items as item}
                <li class="flex items-start gap-2 text-[12px] leading-relaxed" style="color: var(--label-secondary);">
                    <span class="mt-0.5 shrink-0" style="color: {getBulletColor(data.style)};">—</span>
                    <span>{item}</span>
                </li>
            {/each}
        </ul>
    </div>

    <div class="h-px mt-8" style="background: var(--border-subtle);"></div>
</div>

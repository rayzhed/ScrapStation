<script lang="ts">
    import * as LucideIcons from 'lucide-svelte';
    import { onMount } from 'svelte';

    export let title: string;
    export let icon: string;
    export let data: any;

    let visible = false;

    onMount(() => {
        setTimeout(() => visible = true, 300);
    });

    function getIconComponent(iconName: string) {
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        return (LucideIcons as unknown as Record<string, typeof LucideIcons.FileText>)[pascalCase] || LucideIcons.FileText;
    }

    $: IconComponent = getIconComponent(icon);
</script>

<div class="mb-8 transition-all duration-500 {visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}">
    <!-- Section Header -->
    <div class="flex items-center gap-2 mb-4">
        <svelte:component this={IconComponent} size={14} strokeWidth={2} class="text-white/35" />
        <h2 class="text-xs font-bold tracking-[0.12em] text-white/60 uppercase">{title}</h2>
    </div>

    <!-- Content -->
    <div class="p-5 bg-bg-secondary border border-white/5 rounded-subtle">
        <div class="text-xs text-white/65 leading-relaxed whitespace-pre-wrap max-w-3xl">
            {data.content}
        </div>
    </div>

    <div class="h-px bg-white/5 mt-8"></div>
</div>

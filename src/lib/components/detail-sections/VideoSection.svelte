<script lang="ts">
    import * as LucideIcons from 'lucide-svelte';
    import { onMount } from 'svelte';

    export let title: string;
    export let icon: string;
    export let data: any;

    let visible = false;

    onMount(() => {
        setTimeout(() => visible = true, 450);
    });

    function getIconComponent(iconName: string) {
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');
        return (LucideIcons as unknown as Record<string, typeof LucideIcons.PlayCircle>)[pascalCase] || LucideIcons.PlayCircle;
    }

    $: IconComponent = getIconComponent(icon);
</script>

<div class="mb-8 transition-all duration-500 {visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}">
    <!-- Section Header -->
    <div class="flex items-center gap-2 mb-4">
        <svelte:component this={IconComponent} size={14} strokeWidth={2} class="text-white/35" />
        <h2 class="text-xs font-bold tracking-[0.12em] text-white/60 uppercase">{title}</h2>
    </div>

    <!-- Video -->
    <div class="bg-bg-secondary border border-white/5 rounded-subtle p-4">
        <div class="aspect-video overflow-hidden rounded-subtle bg-black border border-white/5">
            <iframe
                src={data.url}
                class="w-full h-full"
                allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
                allowfullscreen
                title="Game trailer"
            ></iframe>
        </div>
    </div>

    <div class="h-px bg-white/5 mt-8"></div>
</div>

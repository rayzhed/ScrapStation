<script lang="ts">
    import { sources, currentSource } from '$lib/stores/sources';
    import { loadGames } from '$lib/stores/games';

    // Import de TOUTES les icônes possibles de Lucide
    import * as LucideIcons from 'lucide-svelte';

    // Fonction pour récupérer l'icône dynamiquement
    function getIconComponent(iconName: string) {
        // Convertir le nom en PascalCase pour Lucide
        // Ex: "wifi" -> "Wifi", "trending-up" -> "TrendingUp"
        const pascalCase = iconName
            .split('-')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join('');

        // @ts-ignore - Accès dynamique aux icônes
        return LucideIcons[pascalCase] || LucideIcons.Gamepad2;
    }

    async function selectSource(id: string) {
        currentSource.set(id);
        await loadGames(id, 1);
    }
</script>

<nav class="border-b border-white/5 bg-bg-primary">
    <div class="max-w-[1920px] mx-auto px-8 py-4 flex gap-2">
        {#each $sources as source}
            {@const IconComponent = getIconComponent(source.icon)}
            <button
                    on:click={() => selectSource(source.id)}
                    class="group relative flex items-center gap-2 px-5 py-2.5 rounded-none font-semibold text-xs tracking-wider transition-all
          {$currentSource === source.id
            ? 'text-white border'
            : 'text-white/50 border border-white/5 hover:text-white/80 hover:bg-white/5'}"
                    style={$currentSource === source.id
          ? `background: ${source.color}15; border-color: ${source.color}40;`
          : ''}
            >
                <svelte:component
                        this={IconComponent}
                        size={18}
                        strokeWidth={2}
                        style={$currentSource === source.id ? `color: ${source.color};` : ''}
                />
                <span>{source.name}</span>
            </button>
        {/each}
    </div>
</nav>
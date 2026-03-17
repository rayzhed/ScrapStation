<script lang="ts">
    import HeroSection from './HeroSection.svelte';
    import VideoSection from './VideoSection.svelte';
    import TextContentSection from './TextContentSection.svelte';
    import MetadataGridSection from './MetadataGridSection.svelte';
    import NumberedStepsSection from './NumberedStepsSection.svelte';
    import AlertBoxSection from './AlertBoxSection.svelte';
    import DownloadButtonsSection from './DownloadButtonsSection.svelte';
    import DynamicSection from './DynamicSection.svelte';

    export let section: any;
    export let sourceId: string;
    export let gameTitle: string = 'Unknown Game';
    export let gameUrl: string = '';
    export let coverUrl: string = '';
    export let archivePassword: string | undefined = undefined;
</script>

{#if section.type === 'hero'}
    <HeroSection data={section.data} />

{:else if section.type === 'video'}
    <VideoSection
            title={section.title}
            icon={section.icon}
            data={section.data}
    />

{:else if section.type === 'text_content'}
    <TextContentSection
            title={section.title}
            icon={section.icon}
            data={section.data}
    />

{:else if section.type === 'metadata_grid'}
    <MetadataGridSection
            title={section.title}
            icon={section.icon}
            data={section.data}
    />

{:else if section.type === 'numbered_steps'}
    <NumberedStepsSection
            title={section.title}
            icon={section.icon}
            data={section.data}
    />

{:else if section.type === 'alert_box'}
    <AlertBoxSection
            title={section.title}
            icon={section.icon}
            data={{ ...section.data, style: section.data?.style ?? section.style }}
    />

{:else if section.type === 'download_buttons'}
    <DownloadButtonsSection
            title={section.title}
            icon={section.icon}
            data={section.data}
            sourceId={sourceId}
            gameTitle={gameTitle}
            gameUrl={gameUrl}
            {coverUrl}
            {archivePassword}
    />

{:else if section.type === 'dynamic'}
    <DynamicSection
            title={section.title}
            icon={section.icon}
            style={section.style}
            data={section.data}
    />
{/if}
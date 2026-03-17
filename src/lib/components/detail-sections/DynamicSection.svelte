<script lang="ts">
    export let title: string;
    export let icon: string;
    export let style: string | undefined = undefined;
    export let data: any;

    $: renderer = data._renderer || 'key_value_grid';
    $: styleConfig = data._style || {};
    $: fields = Object.entries(data).filter(([key]) => !key.startsWith('_'));

    function getBorderColor(v: string): string {
        switch (v) {
            case 'info':    return 'rgba(10,132,255,0.3)';
            case 'warning': return 'rgba(255,159,10,0.3)';
            case 'success': return 'rgba(50,215,75,0.3)';
            case 'error':   return 'rgba(255,69,58,0.3)';
            default:        return 'var(--border)';
        }
    }

    function getBgColor(v: string): string {
        switch (v) {
            case 'info':    return 'rgba(10,132,255,0.05)';
            case 'warning': return 'rgba(255,159,10,0.05)';
            case 'success': return 'rgba(50,215,75,0.05)';
            case 'error':   return 'rgba(255,69,58,0.05)';
            default:        return 'rgba(255,255,255,0.03)';
        }
    }

    $: variant = style || styleConfig.variant || 'default';
</script>

<!-- Section container -->
<div class="mb-6">
    <!-- Section header -->
    {#if title}
        <div class="flex items-center gap-2 mb-4">
            <span>{icon}</span>
            <h3 class="text-[14px] font-semibold" style="color: var(--label-primary);">{title}</h3>
        </div>
    {/if}

    <!-- Dynamic content based on renderer type -->
    <div class="rounded-[8px] p-4" style="border: 1px solid {getBorderColor(variant)}; background: {getBgColor(variant)};">

        {#if renderer === 'key_value_grid'}
            <div class="grid gap-3" style="grid-template-columns: repeat({styleConfig.columns || 2}, 1fr);">
                {#each fields as [key, value]}
                    <div class="flex flex-col gap-1">
                        <span class="text-[11px] uppercase" style="letter-spacing: 0.06em; color: var(--label-tertiary);">{key.replace(/_/g, ' ')}</span>
                        {#if Array.isArray(value)}
                            <span class="text-[13px]" style="color: var(--label-primary);">{value.join(', ')}</span>
                        {:else}
                            <span class="text-[13px]" style="color: var(--label-primary);">{value}</span>
                        {/if}
                    </div>
                {/each}
            </div>

        {:else if renderer === 'list'}
            <ul class="space-y-2">
                {#each fields as [key, value]}
                    {#if Array.isArray(value)}
                        {#each value as item}
                            <li class="flex items-start gap-2 text-[13px]" style="color: var(--label-secondary);">
                                <span style="color: var(--label-tertiary); margin-top: 2px;">•</span>
                                <span>{item}</span>
                            </li>
                        {/each}
                    {:else}
                        <li class="flex items-start gap-2 text-[13px]" style="color: var(--label-secondary);">
                            <span style="color: var(--label-tertiary); margin-top: 2px;">•</span>
                            <span style="color: var(--label-tertiary); margin-right: 4px;">{key}:</span>
                            <span>{value}</span>
                        </li>
                    {/if}
                {/each}
            </ul>

        {:else if renderer === 'text_block'}
            <div>
                {#each fields as [key, value]}
                    {#if typeof value === 'string'}
                        <p class="text-[13px] whitespace-pre-wrap leading-relaxed" style="color: var(--label-secondary);">{value}</p>
                    {/if}
                {/each}
            </div>

        {:else if renderer === 'button_group'}
            <div class="flex flex-wrap gap-2">
                {#each fields as [key, value]}
                    {#if typeof value === 'string' && value.startsWith('http')}
                        <a
                            href={value}
                            target="_blank"
                            rel="noopener noreferrer"
                            class="px-4 py-2 rounded-[6px] text-[13px] transition-colors"
                            style="background: rgba(255,255,255,0.07); border: 1px solid var(--border); color: var(--label-secondary);"
                            on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.11)'; }}
                            on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.07)'; }}
                        >
                            {key.replace(/_/g, ' ')}
                        </a>
                    {/if}
                {/each}
            </div>

        {:else if renderer === 'tags'}
            <div class="flex flex-wrap gap-2">
                {#each fields as [key, value]}
                    {#if Array.isArray(value)}
                        {#each value as tag}
                            <span class="px-3 py-1 rounded-full text-[11px]"
                                  style="background: rgba(255,255,255,0.07); border: 1px solid var(--border-subtle); color: var(--label-secondary);">
                                {tag}
                            </span>
                        {/each}
                    {:else}
                        <span class="px-3 py-1 rounded-full text-[11px]"
                              style="background: rgba(255,255,255,0.07); border: 1px solid var(--border-subtle); color: var(--label-secondary);">
                            {value}
                        </span>
                    {/if}
                {/each}
            </div>

        {:else}
            <div class="text-[12px]" style="color: var(--label-secondary);">
                <p class="mb-2" style="color: var(--label-tertiary);">Unknown renderer: {renderer}</p>
                <pre class="rounded-[6px] p-2 overflow-x-auto text-[11px]"
                     style="background: rgba(0,0,0,0.3); color: var(--label-secondary);">{JSON.stringify(data, null, 2)}</pre>
            </div>
        {/if}
    </div>
</div>

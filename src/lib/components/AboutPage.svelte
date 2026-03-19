<script lang="ts">
    import { Zap, Github, ExternalLink, Heart, Bug, Star } from 'lucide-svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { getVersion } from '@tauri-apps/api/app';
    import { navigateTo } from '$lib/stores/navigation';

    let APP_VERSION = '...';
    getVersion().then(v => APP_VERSION = v);
    const REPO_URL      = 'https://github.com/rayzhed/ScrapStation';
    const ISSUES_URL    = 'https://github.com/rayzhed/ScrapStation/issues';
    const GITHUB_AVATAR = 'https://github.com/rayzhed.png';

    function openLink(url: string) {
        invoke('open_url_in_browser', { url }).catch(() => {});
    }

    const stack = [
        { label: 'Runtime',   value: 'Tauri v2'   },
        { label: 'Frontend',  value: 'SvelteKit'  },
        { label: 'Backend',   value: 'Rust'        },
        { label: 'Language',  value: 'TypeScript'  },
        { label: 'License',   value: 'Custom NC'  },
        { label: 'Platform',  value: 'Windows'    },
    ];

    const links = [
        { icon: Github, label: 'View on GitHub',    action: () => openLink(REPO_URL)    },
        { icon: Bug,    label: 'Report an issue',   action: () => openLink(ISSUES_URL)  },
        { icon: Star,   label: 'Check for updates', action: () => navigateTo('updates') },
    ];

    const legal = [
        {
            title: 'No Content Hosting',
            body: 'ScrapStation hosts nothing. Content is fetched live from sources you install. The author has no access to or control over what those sources serve.',
        },
        {
            title: 'User Responsibility',
            body: 'You are solely responsible for the sources you add and the content you download. This app is a tool — how you use it is on you.',
        },
        {
            title: 'No Endorsement',
            body: 'No third-party source is vetted or endorsed. Always verify a source before installing it — we cannot guarantee its safety or legality.',
        },
        {
            title: 'Privacy',
            body: 'Zero telemetry. No data collection of any kind. Your library, downloads, and settings stay on your machine and never leave it.',
        },
    ];
</script>

<div class="flex flex-col h-full">
    <!-- Header -->
    <!-- Body -->
    <div class="flex-1 flex gap-px overflow-hidden" style="background: var(--border-subtle);">

        <!-- Left panel -->
        <div class="flex flex-col overflow-y-auto"
             style="width: 280px; flex-shrink: 0; background: var(--bg-page);">

            <!-- App identity -->
            <div class="flex flex-col gap-5 p-6 border-b" style="border-color: var(--border-subtle);">
                <div class="flex items-center justify-center rounded-[16px]"
                     style="width: 60px; height: 60px; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1);">
                    <Zap size={28} strokeWidth={1.75} style="color: var(--label-secondary);" />
                </div>
                <div>
                    <p class="text-sm font-bold tracking-widest" style="color: var(--label-primary); letter-spacing: 0.1em;">SCRAPSTATION</p>
                    <p class="text-[11px] font-mono mt-1.5" style="color: var(--label-quaternary);">v{APP_VERSION}</p>
                </div>
                <p class="text-[12px] leading-relaxed" style="color: var(--label-tertiary);">
                    A dynamic, source-driven game launcher. Drop a YAML config to browse, download, and manage games from any compatible source.
                </p>
            </div>

            <!-- Features — fills middle space -->
            <div class="flex-1 p-6 flex flex-col gap-3">
                <p class="text-[10px] font-semibold uppercase tracking-widest mb-1"
                   style="color: var(--label-quaternary); letter-spacing: 0.1em;">Features</p>
                {#each [
                    'Browse games from any YAML-configured source',
                    'Download files with live progress tracking',
                    'Manage and launch your local game library',
                    'Built-in auto-updater for seamless updates',
                ] as feat}
                    <div class="flex items-start gap-2.5">
                        <span style="width:3px;height:3px;border-radius:50%;background:var(--label-quaternary);flex-shrink:0;margin-top:5px;"></span>
                        <span class="text-[12px] leading-snug" style="color: var(--label-tertiary);">{feat}</span>
                    </div>
                {/each}
            </div>

            <!-- Links -->
            <div class="flex flex-col border-t border-b" style="border-color: var(--border-subtle);">
                {#each links as item, i}
                    <button
                        on:click={item.action}
                        class="flex items-center justify-between px-5 py-3.5 cursor-pointer transition-colors hover:bg-white/5 group"
                        style={i < links.length - 1 ? 'border-bottom: 1px solid var(--border-subtle);' : ''}
                    >
                        <div class="flex items-center gap-2.5">
                            <svelte:component this={item.icon} size={13}
                                style="color: var(--label-tertiary); transition: color 0.15s;" />
                            <span class="text-[12px] font-medium transition-colors group-hover:text-white/70"
                                  style="color: var(--label-secondary);">{item.label}</span>
                        </div>
                        <ExternalLink size={11} style="color: var(--label-quaternary); opacity: 0.5;" />
                    </button>
                {/each}
            </div>

            <!-- Footer -->
            <div class="flex items-center justify-center gap-1.5 py-4">
                <span class="text-[11px]" style="color: var(--label-quaternary);">Made with</span>
                <Heart size={9} style="color: #ff453a;" />
                <span class="text-[11px]" style="color: var(--label-quaternary);">by Rayzhed</span>
            </div>
        </div>

        <!-- Right panel -->
        <div class="flex-1 flex flex-col overflow-y-auto" style="background: var(--bg-page);">

            <!-- Author -->
            <div class="p-6 border-b" style="border-color: var(--border-subtle);">
                <p class="text-[10px] font-semibold uppercase tracking-widest mb-5"
                   style="color: var(--label-quaternary); letter-spacing: 0.1em;">Author</p>
                <div class="flex items-center gap-4">
                    <img src={GITHUB_AVATAR} alt="Rayzhed" class="rounded-full shrink-0"
                         style="width: 56px; height: 56px; border: 1px solid rgba(255,255,255,0.1);" />
                    <div>
                        <p class="text-[15px] font-semibold" style="color: var(--label-primary);">Rayzhed</p>
                        <p class="text-[11px] mt-0.5" style="color: var(--label-quaternary);">Developer &amp; maintainer</p>
                        <button
                            on:click={() => openLink('https://github.com/rayzhed')}
                            class="flex items-center gap-1.5 mt-2 cursor-pointer transition-opacity hover:opacity-70"
                        >
                            <Github size={11} style="color: var(--label-quaternary);" />
                            <span class="text-[11px]" style="color: var(--label-quaternary);">@rayzhed</span>
                        </button>
                    </div>
                </div>
            </div>

            <!-- Tech stack -->
            <div class="p-6 border-b" style="border-color: var(--border-subtle);">
                <p class="text-[10px] font-semibold uppercase tracking-widest mb-4"
                   style="color: var(--label-quaternary); letter-spacing: 0.1em;">Tech Stack</p>
                <div class="grid grid-cols-3 gap-3">
                    {#each stack as row}
                        <div class="flex flex-col gap-1.5 px-4 py-3.5 rounded-[10px]"
                             style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.07);">
                            <span class="text-[10px]" style="color: var(--label-quaternary);">{row.label}</span>
                            <span class="text-[12px] font-semibold" style="color: var(--label-secondary);">{row.value}</span>
                        </div>
                    {/each}
                </div>
            </div>

            <!-- License -->
            <div class="p-6 border-b" style="border-color: var(--border-subtle);">
                <p class="text-[10px] font-semibold uppercase tracking-widest mb-4"
                   style="color: var(--label-quaternary); letter-spacing: 0.1em;">License</p>
                <div class="flex flex-col gap-3">
                    <div class="p-4 rounded-[10px]"
                         style="background: rgba(255,255,255,0.02); border: 1px solid rgba(255,255,255,0.06);">
                        <p class="text-[12px] font-semibold mb-2" style="color: var(--label-secondary);">
                            ScrapStation License v1.0
                        </p>
                        <p class="text-[11px] leading-relaxed mb-3" style="color: var(--label-quaternary);">
                            Based on PolyForm Noncommercial License 1.0.0 with additional terms. Personal use, study, and contributions are permitted. Commercial use, redistribution under a different identity, and any claim of authorship are strictly prohibited.
                        </p>
                        <div class="flex flex-wrap gap-1.5">
                            {#each ['Non-commercial', 'No redistribution', 'Attribution required', 'No resale'] as tag}
                                <span class="text-[10px] font-medium px-2 py-0.5 rounded-[4px]"
                                      style="background: rgba(255,159,10,0.08); color: #ff9f0a; border: 1px solid rgba(255,159,10,0.18);">
                                    {tag}
                                </span>
                            {/each}
                        </div>
                    </div>
                    <button
                        on:click={() => openLink('https://github.com/rayzhed/ScrapStation/blob/main/LICENSE')}
                        class="flex items-center justify-between px-4 py-2.5 rounded-[8px] cursor-pointer transition-colors hover:bg-white/5"
                        style="background: rgba(255,255,255,0.02); border: 1px solid rgba(255,255,255,0.06);"
                    >
                        <span class="text-[11px]" style="color: var(--label-tertiary);">Read full license</span>
                        <ExternalLink size={11} style="color: var(--label-quaternary);" />
                    </button>
                </div>
            </div>

            <!-- Legal — fills remaining height -->
            <div class="p-6 flex-1 flex flex-col">
                <p class="text-[10px] font-semibold uppercase tracking-widest mb-4 flex-shrink-0"
                   style="color: var(--label-quaternary); letter-spacing: 0.1em;">Legal</p>
                <div class="grid grid-cols-2 gap-3 flex-1" style="grid-template-rows: 1fr 1fr;">
                    {#each legal as item}
                        <div class="flex flex-col rounded-[8px]"
                             style="background: rgba(255,255,255,0.02); border: 1px solid rgba(255,255,255,0.06);">
                            <p class="text-[11px] font-semibold px-4 pt-4 pb-2 text-center" style="color: var(--label-tertiary);">{item.title}</p>
                            <div class="flex-1 flex items-center justify-center pb-5" style="padding-left: 10%; padding-right: 10%;">
                                <p class="text-[11px] leading-relaxed text-center" style="color: var(--label-quaternary);">{item.body}</p>
                            </div>
                        </div>
                    {/each}
                </div>
            </div>

        </div>
    </div>
</div>

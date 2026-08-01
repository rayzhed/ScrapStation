<script lang="ts">
    import { Zap } from 'lucide-svelte';

    export let phase: 'intro' | 'outro';

    const particles = Array.from({ length: 18 }, (_, i) => ({
        x:    5  + (i * 41 + i * i * 17) % 90,
        y:    5  + (i * 59 + i * i * 11) % 85,
        size: 2  + (i % 4),
        delay: +((i * 0.27) % 3.0).toFixed(2),
        dur:  2.8 + (i % 3) * 0.7,
    }));
</script>

<div class="splash" class:outro={phase === 'outro'} aria-hidden="true">
    {#if phase === 'intro'}
        {#each particles as p}
            <div
                class="particle"
                style="left:{p.x}%;top:{p.y}%;width:{p.size}px;height:{p.size}px;animation-delay:{p.delay}s;animation-duration:{p.dur}s;"
            ></div>
        {/each}
    {/if}

    <div class="body" class:instant={phase === 'outro'}>
        <div class="icon">
            <Zap size={96} strokeWidth={1.25} />
        </div>
        <div class="name">
            {#each 'SCRAPSTATION'.split('') as letter, i}
                <span
                    class="letter"
                    style={phase === 'intro' ? `animation-delay:${320 + i * 42}ms` : ''}
                >{letter}</span>
            {/each}
        </div>
    </div>
</div>

<style>
    .splash {
        position: fixed;
        inset: 0;
        z-index: 9999;
        display: flex;
        align-items: center;
        justify-content: center;
        background: #0a0a0b;
        overflow: hidden;
        pointer-events: none;
        animation: splash-hide 0.5s 1.8s ease forwards;
    }

    .splash.outro {
        opacity: 0;
        pointer-events: all;
        animation: splash-show 0.35s ease forwards;
    }

    .particle {
        position: absolute;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.08);
        animation: float linear infinite;
        pointer-events: none;
    }

    .body {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 20px;
        pointer-events: none;
        user-select: none;
    }

    .icon {
        color: #fff;
        animation: zap-in 0.55s cubic-bezier(0.34, 1.5, 0.64, 1) both;
        pointer-events: all;
        transition: filter 0.45s ease;
    }

    .icon:hover {
        filter:
            drop-shadow(0 0 12px rgba(255, 255, 255, 0.9))
            drop-shadow(0 0 40px rgba(200, 180, 255, 0.6))
            drop-shadow(0 0 90px rgba(140, 100, 255, 0.35));
    }

    .name {
        display: flex;
        letter-spacing: 0.18em;
    }

    .letter {
        font-size: clamp(18px, 3vw, 38px);
        font-weight: 700;
        color: rgba(255, 255, 255, 0.75);
        animation: letter-in 0.4s cubic-bezier(0.34, 1.4, 0.64, 1) both;
    }

    /* Outro: skip per-element animations — parent fade-in does the reveal */
    .instant .icon,
    .instant .letter {
        animation: none;
        opacity: 1;
    }

    @keyframes splash-hide {
        from { opacity: 1; }
        to   { opacity: 0; }
    }

    @keyframes splash-show {
        from { opacity: 0; }
        to   { opacity: 1; }
    }

    @keyframes zap-in {
        0%   { opacity: 0; transform: scale(0.1) rotate(-20deg); filter: none; }
        45%  {
            filter:
                drop-shadow(0 0 20px rgba(255, 255, 255, 1))
                drop-shadow(0 0 60px rgba(200, 180, 255, 0.9))
                drop-shadow(0 0 120px rgba(140, 100, 255, 0.6));
        }
        100% { opacity: 1; transform: scale(1) rotate(0deg); filter: none; }
    }

    @keyframes letter-in {
        0%   { opacity: 0; transform: translateY(12px) scale(0.8); }
        100% { opacity: 1; transform: translateY(0) scale(1); }
    }

    @keyframes float {
        0%   { opacity: 0; transform: translateY(0); }
        15%  { opacity: 0.7; }
        85%  { opacity: 0.2; }
        100% { opacity: 0; transform: translateY(-70px); }
    }
</style>

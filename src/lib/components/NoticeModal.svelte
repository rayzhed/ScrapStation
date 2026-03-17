<script lang="ts">
    import { AlertTriangle, Info, CheckCircle, XCircle, X } from 'lucide-svelte';

    export let notice: {
        id: string;
        trigger: string;
        once?: boolean;
        style?: string;
        title: string;
        message: string;
    };
    export let sourceId: string;
    export let onConfirm: () => void;
    export let onCancel: (() => void) | null = null;

    function getIconComponent(style: string) {
        switch (style) {
            case 'warning': return AlertTriangle;
            case 'danger':  return XCircle;
            case 'success': return CheckCircle;
            default:        return Info;
        }
    }

    function getColor(style: string): string {
        switch (style) {
            case 'warning': return '#ff9f0a';
            case 'danger':  return '#ff453a';
            case 'success': return '#32d74b';
            default:        return '#0a84ff';
        }
    }

    function getBg(style: string): string {
        switch (style) {
            case 'warning': return 'rgba(255,159,10,0.08)';
            case 'danger':  return 'rgba(255,69,58,0.08)';
            case 'success': return 'rgba(50,215,75,0.08)';
            default:        return 'rgba(10,132,255,0.08)';
        }
    }

    function getBorder(style: string): string {
        switch (style) {
            case 'warning': return 'rgba(255,159,10,0.28)';
            case 'danger':  return 'rgba(255,69,58,0.28)';
            case 'success': return 'rgba(50,215,75,0.28)';
            default:        return 'rgba(10,132,255,0.28)';
        }
    }

    $: s = notice.style || 'info';
    $: color = getColor(s);
    $: IconComp = getIconComponent(s);

    function markSeen() {
        if (notice.once) {
            localStorage.setItem(`notice_seen_${sourceId}_${notice.id}`, '1');
        }
    }

    function handleConfirm() {
        markSeen();
        onConfirm();
    }

    function handleCancel() {
        if (onCancel) onCancel();
    }

    function handleBackdropClick() {
        if (onCancel) handleCancel();
        else handleConfirm();
    }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="fixed inset-0 z-[200] flex items-center justify-center"
    style="background: rgba(0,0,0,0.75);"
    on:click={handleBackdropClick}
    on:keydown={(e) => e.key === 'Escape' && handleBackdropClick()}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
>
    <div
        class="relative w-full max-w-sm mx-4 overflow-hidden"
        style="background: var(--bg-surface); border: 1px solid var(--border); border-radius: 12px; box-shadow: 0 24px 64px rgba(0,0,0,0.5);"
        on:click|stopPropagation
        on:keydown|stopPropagation
    >
        <!-- Header -->
        <div class="flex items-center justify-between px-5 pt-5 pb-4" style="border-bottom: 1px solid var(--border-subtle);">
            <div class="flex items-center gap-2.5">
                <div class="w-8 h-8 rounded-[6px] flex items-center justify-center shrink-0"
                     style="background: {getBg(s)}; border: 1px solid {getBorder(s)};">
                    <svelte:component this={IconComp} size={15} strokeWidth={2} style="color: {color};" />
                </div>
                <h2 class="text-[13px] font-semibold" style="color: var(--label-primary); letter-spacing: -0.01em;">{notice.title}</h2>
            </div>
            <button
                on:click={onCancel ? handleCancel : handleConfirm}
                class="p-1.5 rounded-[5px] transition-colors shrink-0"
                style="color: var(--label-tertiary);"
                on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.07)'; (e.currentTarget as HTMLElement).style.color = 'var(--label-primary)'; }}
                on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = 'transparent'; (e.currentTarget as HTMLElement).style.color = 'var(--label-tertiary)'; }}
            >
                <X size={14} />
            </button>
        </div>

        <!-- Message -->
        <div class="px-5 py-4">
            <p class="text-[12px] leading-relaxed" style="color: var(--label-secondary);">{notice.message}</p>
        </div>

        <!-- Actions -->
        <div class="px-5 pb-5 flex gap-2">
            {#if onCancel}
                <button
                    on:click={handleCancel}
                    class="btn-secondary flex-1"
                    style="justify-content: center;"
                >
                    Cancel
                </button>
            {/if}
            <button
                on:click={handleConfirm}
                class="flex-1 flex items-center justify-center gap-1.5 transition-colors"
                style="padding: 8px 16px; border-radius: 8px; font-size: 12px; font-weight: 700; background: #f5f5f7; color: #0a0a0b; border: none;"
                on:mouseenter={e => { (e.currentTarget as HTMLElement).style.background = '#e5e5e7'; }}
                on:mouseleave={e => { (e.currentTarget as HTMLElement).style.background = '#f5f5f7'; }}
            >
                {onCancel ? 'Continue' : 'Got it'}
            </button>
        </div>
    </div>
</div>

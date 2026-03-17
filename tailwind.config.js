/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'ui-monospace', 'monospace'],
      },
      colors: {
        // Backgrounds — clear layered depth
        bg: {
          page:     '#0a0a0b',   // page chrome
          sidebar:  '#111114',   // sidebar (visible separation from page)
          surface:  '#1a1a1c',   // cards, panels
          elevated: '#252527',   // dropdowns, tooltips
          input:    'rgba(255,255,255,0.06)',
          // legacy aliases
          primary:  '#0a0a0b',
          secondary:'#1a1a1c',
          tertiary: '#252527',
        },
        // Text — all WCAG AA on #0a0a0b (≥ 4.5:1)
        label: {
          primary:   '#f5f5f7',               // 19.6:1 ✓
          secondary: 'rgba(245,245,247,0.65)', // 8.7:1 ✓
          tertiary:  'rgba(245,245,247,0.50)', // 5.6:1 ✓
          quarternary:'rgba(245,245,247,0.35)',// 3.1:1 — disabled / decorative only
        },
        // Borders
        border: {
          subtle:  'rgba(255,255,255,0.08)',
          default: 'rgba(255,255,255,0.10)',
          strong:  'rgba(255,255,255,0.16)',
        },
        // System accents (Apple iOS dark mode exact)
        system: {
          blue:   '#0a84ff',
          green:  '#32d74b',
          red:    '#ff453a',
          orange: '#ff9f0a',
          purple: '#bf5af2',
          yellow: '#ffd60a',
          gray:   '#636366',
        },
        // Legacy compatibility
        accent: {
          primary: '#f5f5f7',
          muted:   'rgba(245,245,247,0.65)',
          danger:  '#ff453a',
          success: '#32d74b',
          warning: '#ff9f0a',
        },
      },
      borderRadius: {
        card:   '12px',
        panel:  '14px',
        dialog: '16px',
        sm:     '6px',
        md:     '8px',
        lg:     '10px',
        // legacy
        subtle: '8px',
        xs:     '4px',
      },
      boxShadow: {
        // Clean single-layer shadows — no color glows
        'sm':    '0 1px 2px rgba(0,0,0,0.4)',
        'card':  '0 1px 3px rgba(0,0,0,0.5), 0 4px 12px rgba(0,0,0,0.3)',
        'panel': '0 4px 24px rgba(0,0,0,0.5), 0 1px 4px rgba(0,0,0,0.4)',
        'dialog':'0 8px 40px rgba(0,0,0,0.7), 0 2px 8px rgba(0,0,0,0.5)',
        'inset': 'inset 0 1px 0 rgba(255,255,255,0.06)',
        // legacy
        'card-hover': '0 4px 16px rgba(0,0,0,0.5), 0 1px 4px rgba(0,0,0,0.5)',
      },
      fontSize: {
        // Apple macOS type scale
        'footnote': ['11px', { lineHeight: '16px', letterSpacing: '0' }],
        'caption':  ['12px', { lineHeight: '16px', letterSpacing: '0' }],
        'body':     ['13px', { lineHeight: '20px', letterSpacing: '-0.01em' }],
        'callout':  ['14px', { lineHeight: '21px', letterSpacing: '-0.01em' }],
        'headline': ['15px', { lineHeight: '22px', letterSpacing: '-0.015em' }],
        'title3':   ['17px', { lineHeight: '24px', letterSpacing: '-0.02em' }],
        'title2':   ['20px', { lineHeight: '28px', letterSpacing: '-0.025em' }],
        'title1':   ['24px', { lineHeight: '32px', letterSpacing: '-0.03em' }],
        // keep xs/sm for tailwind compat
        '2xs': ['10px', { lineHeight: '14px' }],
        'xs':  ['11px', { lineHeight: '16px' }],
        'sm':  ['12px', { lineHeight: '18px' }],
        'base':['13px', { lineHeight: '20px' }],
        'md':  ['14px', { lineHeight: '21px' }],
        'lg':  ['15px', { lineHeight: '22px' }],
        'xl':  ['17px', { lineHeight: '24px' }],
        '2xl': ['20px', { lineHeight: '28px' }],
      },
    },
  },
  plugins: [],
}

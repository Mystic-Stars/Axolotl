import type { Config } from 'tailwindcss'

export default {
    darkMode: ['class'],
    content: [
        './app.vue',
        './components/**/*.{vue,ts}',
        './composables/**/*.ts',
        './layouts/**/*.vue',
        './pages/**/*.vue',
    ],
    theme: {
        extend: {
            colors: {
                surface: {
                    1: 'hsl(var(--surface-1))',
                    2: 'hsl(var(--surface-2))',
                    3: 'hsl(var(--surface-3))',
                    4: 'hsl(var(--surface-4))',
                    5: 'hsl(var(--surface-5))',
                },
                border: 'hsl(var(--border))',
                input: 'hsl(var(--input))',
                ring: 'hsl(var(--ring))',
                background: 'hsl(var(--background))',
                foreground: 'hsl(var(--foreground))',
                primary: {
                    DEFAULT: 'hsl(var(--primary))',
                    foreground: 'hsl(var(--primary-foreground))',
                },
                secondary: {
                    DEFAULT: 'hsl(var(--secondary))',
                    foreground: 'hsl(var(--secondary-foreground))',
                },
                muted: { DEFAULT: 'hsl(var(--muted))', foreground: 'hsl(var(--muted-foreground))' },
                accent: {
                    DEFAULT: 'hsl(var(--accent))',
                    foreground: 'hsl(var(--accent-foreground))',
                },
                destructive: {
                    DEFAULT: 'hsl(var(--destructive))',
                    foreground: 'hsl(var(--destructive-foreground))',
                },
                card: { DEFAULT: 'hsl(var(--card))', foreground: 'hsl(var(--card-foreground))' },
                popover: {
                    DEFAULT: 'hsl(var(--popover))',
                    foreground: 'hsl(var(--popover-foreground))',
                },
            },
            borderRadius: {
                lg: '8px',
                md: '6px',
                sm: '4px',
            },
            fontFamily: {
                sans: [
                    'Inter',
                    'PingFang SC',
                    'Microsoft YaHei',
                    'ui-sans-serif',
                    'system-ui',
                    'sans-serif',
                ],
                mono: ['JetBrains Mono', 'ui-monospace', 'SFMono-Regular', 'monospace'],
            },
        },
    },
    plugins: [],
} satisfies Config

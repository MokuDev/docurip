/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        // Background tones (theme-aware via CSS variables, see src/styles/index.css)
        deepVoid: 'rgb(var(--color-deep-void) / <alpha-value>)',
        surface: 'rgb(var(--color-surface) / <alpha-value>)',
        abyssal: 'rgb(var(--color-abyssal) / <alpha-value>)',

        // Cyberpunk accent
        accentGreen: '#16E08D',
        brightGreen: '#22F29D',
        toxicGreen: '#0AAF6D',

        // Danger / warning
        crimson: '#E8465F',
        amber: '#F59E0B',

        // Blue secondary
        cyberBlue: '#3B82F6',
        cyan: '#06B6D4',

        // Text (theme-aware via CSS variables, see src/styles/index.css)
        ghost: 'rgb(var(--color-ghost) / <alpha-value>)',
        smooth: 'rgb(var(--color-smooth) / <alpha-value>)',
        secondary: 'rgb(var(--color-secondary) / <alpha-value>)',
        charcoal: 'rgb(var(--color-charcoal) / <alpha-value>)',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
        display: ['Inter', 'system-ui', 'sans-serif'],
      },
      transitionDuration: {
        fast: '150ms',
        slow: '300ms',
      },
      boxShadow: {
        glow: '0 0 20px rgba(22, 224, 141, 0.15)',
      },
    },
  },
  plugins: [],
};

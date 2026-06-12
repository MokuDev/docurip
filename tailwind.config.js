/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        // Background tones
        deepVoid: '#080C14',
        surface: '#0F172A',
        abyssal: '#1E293B',

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

        // Text
        ghost: '#F1F5F9',
        smooth: '#CBD5E1',
        secondary: '#94A3B8',
        charcoal: '#64748B',
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

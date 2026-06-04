/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'bg-dark': '#050505',
        'neon-blue': '#00f0ff',
        'neon-purple': '#b026ff',
        'neon-green': '#00ff88',
        'neon-red': '#ff3366',
        'neon-gold': '#ffd700',
      },
    },
  },
  plugins: [],
}

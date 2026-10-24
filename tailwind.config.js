/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,ts}"],
  theme: {
    extend: {
      colors: {
        // Black surfaces with a faint green undertone; one green accent.
        ink: { DEFAULT: "#070908", raised: "#0d110f", line: "#1b2420" },
        accent: { DEFAULT: "#22c55e", soft: "#4ade80", deep: "#15803d" },
      },
      fontFamily: {
        sans: ["Inter", "Segoe UI", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Cascadia Mono", "Consolas", "ui-monospace", "monospace"],
      },
      keyframes: {
        backdrop_pulse: {
          "0%, 100%": { transform: "scale(1)", opacity: ".55" },
          "50%": { transform: "scale(1.25)", opacity: ".95" },
        },
      },
      animation: { "backdrop-pulse": "backdrop_pulse 2.4s ease-in-out infinite" },
    },
  },
  plugins: [],
};

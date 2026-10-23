/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,ts}"],
  theme: {
    extend: {
      colors: {
        obsidian: "#0a0b10",
        cyan: { neon: "#00F2FE" },
        violet: { neon: "#7000FF" },
      },
      fontFamily: {
        sans: ["Inter", "Segoe UI", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Cascadia Mono", "Consolas", "ui-monospace", "monospace"],
      },
      boxShadow: {
        "glow-cyan": "0 0 18px rgba(0,242,254,.35)",
        "glow-violet": "0 0 18px rgba(112,0,255,.4)",
      },
      keyframes: {
        ping_pulse: {
          "0%": { transform: "scale(1)", opacity: ".7" },
          "100%": { transform: "scale(1.9)", opacity: "0" },
        },
      },
      animation: { "ping-pulse": "ping_pulse 1.8s cubic-bezier(0,0,.2,1) infinite" },
    },
  },
  plugins: [],
};

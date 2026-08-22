/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        cv: {
          bg: "var(--cv-bg)",
          panel: "var(--cv-panel)",
          "panel-2": "var(--cv-panel-2)",
          "panel-3": "var(--cv-panel-3)",
          text: "var(--cv-text)",
          subtext: "var(--cv-subtext)",
          accent: "var(--cv-accent)",
          "accent-2": "var(--cv-accent-2)",
          "accent-3": "var(--cv-accent-3)",
          gold: "var(--cv-gold)",
          "row-a": "var(--cv-row-a)",
          "row-b": "var(--cv-row-b)",
          danger: "var(--cv-danger)",
          "neon-1": "var(--cv-neon-1)",
          "neon-2": "var(--cv-neon-2)",
          "neon-3": "var(--cv-neon-3)",
        },
      },
      fontFamily: {
        display: ["Inter", "SF Pro Display", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Fira Code", "monospace"],
      },
      backdropBlur: {
        xs: "2px",
        "4xl": "72px",
      },
      animation: {
        "pulse-glow": "pulse-glow 2s ease-in-out infinite",
        "slide-in": "slide-in 0.3s ease-out",
        "fade-in": "fade-in 0.4s ease-out",
        shimmer: "shimmer 2s linear infinite",
        float: "float 6s ease-in-out infinite",
        "comet": "comet 8s linear infinite",
      },
      keyframes: {
        "pulse-glow": {
          "0%, 100%": { boxShadow: "0 0 5px var(--cv-accent), 0 0 20px transparent" },
          "50%": { boxShadow: "0 0 20px var(--cv-accent), 0 0 40px var(--cv-accent-2)" },
        },
        "slide-in": {
          "0%": { transform: "translateX(-20px)", opacity: "0" },
          "100%": { transform: "translateX(0)", opacity: "1" },
        },
        "fade-in": {
          "0%": { opacity: "0", transform: "translateY(8px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        shimmer: {
          "0%": { backgroundPosition: "-200% 0" },
          "100%": { backgroundPosition: "200% 0" },
        },
        float: {
          "0%, 100%": { transform: "translateY(0px)" },
          "50%": { transform: "translateY(-10px)" },
        },
        comet: {
          "0%": { transform: "translateX(-100vw) translateY(50vh)", opacity: "0" },
          "10%": { opacity: "1" },
          "90%": { opacity: "1" },
          "100%": { transform: "translateX(100vw) translateY(-50vh)", opacity: "0" },
        },
      },
    },
  },
  plugins: [],
};

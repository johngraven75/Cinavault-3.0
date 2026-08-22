export const v2FutureDesign = {
  name: "CinaVault Future Horizon",
  version: "v2-build-1",
  palette: {
    background: "#f7fbff",
    surface: "rgba(255,255,255,0.82)",
    accent: "#00d9ff",
    secondary: "#7c3cff",
    highlight: "#ffffff",
    text: "#07111f",
  },
  motion: {
    pageTransitionMs: 420,
    cardHoverMs: 220,
    spring: {
      stiffness: 420,
      damping: 32,
    },
  },
  effects: {
    glass: true,
    holographicPanels: true,
    ambientLighting: true,
    cinematicTransitions: true,
  },
} as const;

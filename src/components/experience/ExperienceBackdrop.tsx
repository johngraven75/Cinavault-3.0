import { useEffect, useState } from "react";
import { motion, useMotionValue, useSpring, useTransform } from "framer-motion";

const PARTICLES = Array.from({ length: 28 }, (_, index) => ({
  id: index,
  left: `${(index * 37) % 100}%`,
  top: `${(index * 61) % 100}%`,
  size: 1 + (index % 3),
  delay: (index % 9) * 0.45,
  duration: 5 + (index % 7),
}));

export default function ExperienceBackdrop() {
  const pointerX = useMotionValue(0.5);
  const pointerY = useMotionValue(0.5);
  const smoothX = useSpring(pointerX, { stiffness: 42, damping: 18 });
  const smoothY = useSpring(pointerY, { stiffness: 42, damping: 18 });
  const driftX = useTransform(smoothX, [0, 1], [-36, 36]);
  const driftY = useTransform(smoothY, [0, 1], [-24, 24]);
  const reverseX = useTransform(smoothX, [0, 1], [26, -26]);
  const reverseY = useTransform(smoothY, [0, 1], [18, -18]);
  const [reducedMotion, setReducedMotion] = useState(false);

  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const updatePreference = () => setReducedMotion(query.matches);
    updatePreference();
    query.addEventListener("change", updatePreference);

    const handlePointer = (event: PointerEvent) => {
      pointerX.set(event.clientX / Math.max(window.innerWidth, 1));
      pointerY.set(event.clientY / Math.max(window.innerHeight, 1));
    };
    window.addEventListener("pointermove", handlePointer, { passive: true });
    return () => {
      query.removeEventListener("change", updatePreference);
      window.removeEventListener("pointermove", handlePointer);
    };
  }, [pointerX, pointerY]);

  return (
    <div className="cv-experience-backdrop" aria-hidden="true">
      <div className="cv-deep-space" />
      <motion.div
        className="cv-aurora cv-aurora-a"
        style={reducedMotion ? undefined : { x: driftX, y: driftY }}
      />
      <motion.div
        className="cv-aurora cv-aurora-b"
        style={reducedMotion ? undefined : { x: reverseX, y: reverseY }}
      />
      <motion.div
        className="cv-aurora cv-aurora-c"
        style={reducedMotion ? undefined : { x: driftX, y: reverseY }}
      />

      <div className="cv-orbit-system cv-orbit-system-a">
        <div className="cv-orbit-ring cv-orbit-ring-one" />
        <div className="cv-orbit-ring cv-orbit-ring-two" />
        <div className="cv-orbit-node cv-orbit-node-one" />
        <div className="cv-orbit-node cv-orbit-node-two" />
      </div>
      <div className="cv-orbit-system cv-orbit-system-b">
        <div className="cv-orbit-ring cv-orbit-ring-one" />
        <div className="cv-orbit-ring cv-orbit-ring-two" />
        <div className="cv-orbit-node cv-orbit-node-one" />
        <div className="cv-orbit-node cv-orbit-node-two" />
      </div>

      <div className="cv-data-grid" />
      <div className="cv-horizon-line" />
      <div className="cv-scan-beam" />

      <div className="cv-particle-field">
        {PARTICLES.map((particle) => (
          <motion.span
            key={particle.id}
            className="cv-particle"
            style={{
              left: particle.left,
              top: particle.top,
              width: particle.size,
              height: particle.size,
            }}
            animate={
              reducedMotion
                ? { opacity: 0.35 }
                : {
                    opacity: [0.08, 0.75, 0.12],
                    y: [0, -18, 0],
                    scale: [0.8, 1.5, 0.8],
                  }
            }
            transition={{
              duration: particle.duration,
              delay: particle.delay,
              repeat: Infinity,
              ease: "easeInOut",
            }}
          />
        ))}
      </div>
      <div className="cv-film-grain" />
    </div>
  );
}

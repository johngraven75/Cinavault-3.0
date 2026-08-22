import React, { useRef, useEffect } from "react";

interface Props {
  className?: string;
  meteorCount?: number;
}

type Star = {
  x: number;
  y: number;
  r: number;
  alpha: number;
  shimmer: number;
};

type Comet = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  length: number;
  radius: number;
  hue: number;
  warm: number;
  age: number;
  life: number;
  delay: number;
};

function makeStars(count: number): Star[] {
  return Array.from({ length: count }, () => ({
    x: Math.random(),
    y: Math.random(),
    r: Math.random() * 1.35 + 0.25,
    alpha: Math.random() * 0.75 + 0.2,
    shimmer: Math.random() * Math.PI * 2,
  }));
}

function resetComet(comet: Comet, width: number, height: number) {
  const foreground = Math.random() > 0.82;
  const speed = foreground
    ? Math.random() * 5.5 + 10.5
    : Math.random() * 5.8 + 6.8;
  const angle = (Math.random() * 15 + 24) * (Math.PI / 180);
  const startsFromTop = Math.random() > 0.28;

  comet.x = startsFromTop
    ? Math.random() * width * 1.25 - width * 0.15
    : -width * 0.18;
  comet.y = startsFromTop
    ? -height * (Math.random() * 0.55 + 0.08)
    : Math.random() * height * 0.36;
  comet.vx = Math.cos(angle) * speed;
  comet.vy = Math.sin(angle) * speed;
  comet.length = foreground
    ? Math.random() * 260 + 300
    : Math.random() * 160 + 150;
  comet.radius = foreground
    ? Math.random() * 1.8 + 2.5
    : Math.random() * 1.4 + 1.1;
  comet.hue = Math.random() * 42 + 188;
  comet.warm = Math.random() * 32 + 22;
  comet.age = 0;
  comet.life = foreground ? Math.random() * 46 + 78 : Math.random() * 80 + 95;
  comet.delay = Math.random() * 95;
}

function makeComets(count: number, width: number, height: number): Comet[] {
  return Array.from({ length: count }, (_, index) => {
    const comet = {
      x: 0,
      y: 0,
      vx: 0,
      vy: 0,
      length: 0,
      radius: 0,
      hue: 200,
      warm: 30,
      age: 0,
      life: 120,
      delay: 0,
    };
    resetComet(comet, width, height);
    if (index < Math.min(5, count)) {
      comet.x = width * (0.08 + Math.random() * 0.82);
      comet.y = height * (0.02 + Math.random() * 0.46);
      comet.age = comet.life * (0.12 + Math.random() * 0.4);
      comet.delay = 0;
    }
    return comet;
  });
}

export default function MeteorShower({
  className = "",
  meteorCount = 18,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let width = 1;
    let height = 1;
    let frame = 0;
    let animId = 0;
    let stars = makeStars(180);
    let comets: Comet[] = [];
    const pixelRatio = Math.min(window.devicePixelRatio || 1, 2);

    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      width = Math.max(1, rect.width);
      height = Math.max(1, rect.height);
      canvas.width = Math.floor(width * pixelRatio);
      canvas.height = Math.floor(height * pixelRatio);
      ctx.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
      stars = makeStars(Math.max(120, Math.round((width * height) / 1400)));
      comets = makeComets(meteorCount, width, height);
    };

    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    resize();

    const drawSky = () => {
      const night = ctx.createLinearGradient(0, 0, 0, height);
      night.addColorStop(0, "#02030a");
      night.addColorStop(0.36, "#07142a");
      night.addColorStop(0.68, "#14203d");
      night.addColorStop(1, "#03050c");
      ctx.fillStyle = night;
      ctx.fillRect(0, 0, width, height);

      const milkyWay = ctx.createLinearGradient(
        -width * 0.15,
        height,
        width,
        -height * 0.08,
      );
      milkyWay.addColorStop(0, "rgba(16, 41, 76, 0)");
      milkyWay.addColorStop(0.38, "rgba(74, 113, 146, 0.16)");
      milkyWay.addColorStop(0.52, "rgba(197, 151, 111, 0.11)");
      milkyWay.addColorStop(0.72, "rgba(64, 119, 152, 0.12)");
      milkyWay.addColorStop(1, "rgba(16, 41, 76, 0)");
      ctx.fillStyle = milkyWay;
      ctx.fillRect(0, 0, width, height);

      ctx.save();
      ctx.filter = "blur(22px)";
      const warmHaze = ctx.createRadialGradient(
        width * 0.82,
        height * 0.78,
        0,
        width * 0.82,
        height * 0.78,
        width * 0.72,
      );
      warmHaze.addColorStop(0, "rgba(246, 142, 73, 0.26)");
      warmHaze.addColorStop(0.42, "rgba(55, 108, 154, 0.12)");
      warmHaze.addColorStop(1, "rgba(5, 7, 14, 0)");
      ctx.fillStyle = warmHaze;
      ctx.fillRect(0, 0, width, height);
      ctx.restore();

      ctx.globalCompositeOperation = "screen";
      for (const star of stars) {
        const twinkle =
          star.alpha + Math.sin(frame * 0.025 + star.shimmer) * 0.18;
        ctx.beginPath();
        ctx.fillStyle = `rgba(235, 248, 255, ${Math.max(0.08, twinkle)})`;
        ctx.arc(star.x * width, star.y * height, star.r, 0, Math.PI * 2);
        ctx.fill();
        if (star.r > 1.25 && twinkle > 0.7) {
          ctx.beginPath();
          ctx.fillStyle = `rgba(183, 219, 255, ${0.11 * twinkle})`;
          ctx.arc(
            star.x * width,
            star.y * height,
            star.r * 4.2,
            0,
            Math.PI * 2,
          );
          ctx.fill();
        }
      }
      ctx.globalCompositeOperation = "source-over";
    };

    const drawComet = (comet: Comet) => {
      if (comet.delay > 0) {
        comet.delay -= 1;
        return;
      }

      comet.x += comet.vx;
      comet.y += comet.vy;
      comet.age += 1;

      if (
        comet.age > comet.life ||
        comet.x > width + comet.length ||
        comet.y > height + comet.length
      ) {
        resetComet(comet, width, height);
        return;
      }

      const progress = comet.age / comet.life;
      const alpha = Math.sin(progress * Math.PI);
      const speed = Math.hypot(comet.vx, comet.vy);
      const ux = comet.vx / speed;
      const uy = comet.vy / speed;
      const tailX = comet.x - ux * comet.length;
      const tailY = comet.y - uy * comet.length;

      ctx.globalCompositeOperation = "lighter";

      const smoke = ctx.createLinearGradient(comet.x, comet.y, tailX, tailY);
      smoke.addColorStop(0, `hsla(${comet.warm}, 100%, 78%, ${0.32 * alpha})`);
      smoke.addColorStop(
        0.18,
        `hsla(${comet.hue}, 100%, 72%, ${0.24 * alpha})`,
      );
      smoke.addColorStop(
        0.72,
        `hsla(${comet.hue + 38}, 100%, 62%, ${0.07 * alpha})`,
      );
      smoke.addColorStop(1, "rgba(0, 0, 0, 0)");
      ctx.strokeStyle = smoke;
      ctx.lineWidth = comet.radius * 5.2;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(comet.x, comet.y);
      ctx.lineTo(tailX, tailY);
      ctx.stroke();

      const ion = ctx.createLinearGradient(comet.x, comet.y, tailX, tailY);
      ion.addColorStop(0, `rgba(255, 255, 255, ${0.95 * alpha})`);
      ion.addColorStop(0.16, `hsla(${comet.hue}, 100%, 76%, ${0.68 * alpha})`);
      ion.addColorStop(1, "rgba(0, 0, 0, 0)");
      ctx.strokeStyle = ion;
      ctx.lineWidth = comet.radius * 1.15;
      ctx.beginPath();
      ctx.moveTo(comet.x, comet.y);
      ctx.lineTo(tailX, tailY);
      ctx.stroke();

      const head = ctx.createRadialGradient(
        comet.x,
        comet.y,
        0,
        comet.x,
        comet.y,
        comet.radius * 8.5,
      );
      head.addColorStop(0, `rgba(255, 255, 255, ${0.9 * alpha})`);
      head.addColorStop(
        0.22,
        `hsla(${comet.warm}, 100%, 72%, ${0.68 * alpha})`,
      );
      head.addColorStop(0.55, `hsla(${comet.hue}, 100%, 62%, ${0.18 * alpha})`);
      head.addColorStop(1, "rgba(0, 0, 0, 0)");
      ctx.fillStyle = head;
      ctx.beginPath();
      ctx.arc(comet.x, comet.y, comet.radius * 8.5, 0, Math.PI * 2);
      ctx.fill();

      const core = ctx.createRadialGradient(
        comet.x,
        comet.y,
        0,
        comet.x,
        comet.y,
        comet.radius * 2.2,
      );
      core.addColorStop(0, `rgba(255, 255, 255, ${alpha})`);
      core.addColorStop(0.42, `rgba(255, 208, 158, ${0.72 * alpha})`);
      core.addColorStop(1, "rgba(255, 208, 158, 0)");
      ctx.fillStyle = core;
      ctx.beginPath();
      ctx.arc(comet.x, comet.y, comet.radius * 2.2, 0, Math.PI * 2);
      ctx.fill();

      for (let i = 0; i < 4; i += 1) {
        const drift = (i + 1) * comet.radius * 5.5;
        const side = i % 2 === 0 ? 1 : -1;
        const fragmentX =
          comet.x - ux * drift + -uy * side * comet.radius * (2.2 + i);
        const fragmentY =
          comet.y - uy * drift + ux * side * comet.radius * (2.2 + i);
        ctx.fillStyle = `rgba(255, 224, 188, ${0.2 * alpha})`;
        ctx.beginPath();
        ctx.arc(
          fragmentX,
          fragmentY,
          Math.max(0.9, comet.radius * (0.58 - i * 0.08)),
          0,
          Math.PI * 2,
        );
        ctx.fill();
      }

      ctx.globalCompositeOperation = "source-over";
    };

    const drawAtmosphere = () => {
      ctx.save();
      ctx.filter = "blur(18px)";
      const groundGlow = ctx.createLinearGradient(0, height * 0.52, 0, height);
      groundGlow.addColorStop(0, "rgba(0, 0, 0, 0)");
      groundGlow.addColorStop(0.68, "rgba(26, 50, 74, 0.2)");
      groundGlow.addColorStop(1, "rgba(8, 12, 20, 0.86)");
      ctx.fillStyle = groundGlow;
      ctx.fillRect(0, 0, width, height);
      ctx.restore();

      const haze = ctx.createLinearGradient(0, height * 0.55, 0, height);
      haze.addColorStop(0, "rgba(0, 0, 0, 0)");
      haze.addColorStop(0.72, "rgba(6, 13, 27, 0.42)");
      haze.addColorStop(1, "rgba(0, 0, 0, 0.82)");
      ctx.fillStyle = haze;
      ctx.fillRect(0, 0, width, height);

      ctx.fillStyle = "rgba(0, 0, 0, 0.38)";
      ctx.beginPath();
      ctx.moveTo(0, height);
      ctx.lineTo(0, height * 0.84);
      ctx.bezierCurveTo(
        width * 0.18,
        height * 0.78,
        width * 0.28,
        height * 0.86,
        width * 0.42,
        height * 0.8,
      );
      ctx.bezierCurveTo(
        width * 0.56,
        height * 0.73,
        width * 0.73,
        height * 0.88,
        width,
        height * 0.78,
      );
      ctx.lineTo(width, height);
      ctx.closePath();
      ctx.fill();
    };

    const draw = () => {
      frame += 1;
      drawSky();
      for (const comet of comets) drawComet(comet);
      drawAtmosphere();
      animId = requestAnimationFrame(draw);
    };

    draw();

    return () => {
      cancelAnimationFrame(animId);
      observer.disconnect();
    };
  }, [meteorCount]);

  return (
    <canvas
      ref={canvasRef}
      className={`absolute inset-0 z-[1] h-full w-full pointer-events-none ${className}`}
      aria-hidden="true"
    />
  );
}

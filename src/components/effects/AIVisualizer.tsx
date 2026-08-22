// CinaVault Premium — AI Activity Visualizer (Rotating Ring + Waveform + Comet)
import React, { useRef, useEffect } from "react";

interface Props {
  active?: boolean;
  className?: string;
}

export default function AIVisualizer({
  active = false,
  className = "",
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d")!;
    let animId: number;
    let t = 0;

    const resize = () => {
      canvas.width = canvas.offsetWidth * 2;
      canvas.height = canvas.offsetHeight * 2;
      ctx.scale(2, 2);
    };
    resize();

    const draw = () => {
      const w = canvas.offsetWidth;
      const h = canvas.offsetHeight;
      const cx = w / 2;
      const cy = h / 2;
      t += 0.02;

      ctx.clearRect(0, 0, w, h);

      const intensity = active ? 1 : 0.25;

      // Rotating ring
      const ringR = Math.min(w, h) * 0.3;
      ctx.save();
      ctx.translate(cx, cy);
      ctx.rotate(t * 0.5);
      for (let i = 0; i < 36; i++) {
        const angle = (i / 36) * Math.PI * 2;
        const dotSize = 1.5 + Math.sin(t * 3 + i * 0.5) * 1 * intensity;
        const alpha = 0.2 + Math.sin(t * 2 + i * 0.3) * 0.3 * intensity;
        const x = Math.cos(angle) * ringR;
        const y = Math.sin(angle) * ringR;
        ctx.beginPath();
        ctx.arc(x, y, dotSize, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(167,139,250,${alpha})`;
        ctx.fill();
      }
      ctx.restore();

      // Waveform bars
      const barCount = 32;
      const barWidth = (w * 0.6) / barCount;
      const barStartX = w * 0.2;
      const barBaseY = cy + ringR + 30;

      for (let i = 0; i < barCount; i++) {
        const freq = active
          ? Math.sin(t * 4 + i * 0.4) * 0.8 + Math.sin(t * 7 + i * 0.7) * 0.3
          : Math.sin(t + i * 0.3) * 0.15;
        const barH = Math.abs(freq) * 25 * intensity + 2;
        const x = barStartX + i * barWidth;
        const alpha = 0.3 + Math.abs(freq) * 0.5;

        const grad = ctx.createLinearGradient(x, barBaseY, x, barBaseY - barH);
        grad.addColorStop(0, `rgba(167,139,250,${alpha * 0.3})`);
        grad.addColorStop(1, `rgba(192,132,252,${alpha})`);

        ctx.fillStyle = grad;
        ctx.fillRect(x, barBaseY - barH, barWidth - 1, barH);
        ctx.fillRect(x, barBaseY + 1, barWidth - 1, barH * 0.3);
      }

      // Sweep line
      if (active) {
        const sweepX = (Math.sin(t * 1.5) * 0.5 + 0.5) * w;
        const grad = ctx.createLinearGradient(sweepX - 30, 0, sweepX + 30, 0);
        grad.addColorStop(0, "transparent");
        grad.addColorStop(0.5, "rgba(167,139,250,0.15)");
        grad.addColorStop(1, "transparent");
        ctx.fillStyle = grad;
        ctx.fillRect(sweepX - 30, 0, 60, h);
      }

      // Center orb
      const orbGlow = ctx.createRadialGradient(
        cx,
        cy,
        0,
        cx,
        cy,
        20 + intensity * 10,
      );
      orbGlow.addColorStop(0, `rgba(192,132,252,${0.4 * intensity})`);
      orbGlow.addColorStop(0.5, `rgba(167,139,250,${0.15 * intensity})`);
      orbGlow.addColorStop(1, "transparent");
      ctx.fillStyle = orbGlow;
      ctx.fillRect(cx - 40, cy - 40, 80, 80);

      ctx.beginPath();
      ctx.arc(cx, cy, 4 + intensity * 3, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255,255,255,${0.5 + intensity * 0.4})`;
      ctx.fill();

      animId = requestAnimationFrame(draw);
    };

    draw();
    return () => cancelAnimationFrame(animId);
  }, [active]);

  return <canvas ref={canvasRef} className={`w-full h-full ${className}`} />;
}

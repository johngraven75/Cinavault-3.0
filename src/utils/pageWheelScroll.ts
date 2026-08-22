const WHEEL_LINE_HEIGHT = 40;

export function getWheelDeltaPixels(
  deltaY: number,
  deltaMode: number,
  pageHeight: number,
): number {
  if (!Number.isFinite(deltaY)) return 0;
  if (deltaMode === 1) return deltaY * WHEEL_LINE_HEIGHT;
  if (deltaMode === 2) return deltaY * pageHeight;
  return deltaY;
}

export function getWheelScrolledTop(
  currentTop: number,
  deltaPixels: number,
  scrollHeight: number,
  clientHeight: number,
): number {
  const maxTop = Math.max(0, scrollHeight - clientHeight);
  const nextTop = currentTop + deltaPixels;
  return Math.max(0, Math.min(maxTop, nextTop));
}

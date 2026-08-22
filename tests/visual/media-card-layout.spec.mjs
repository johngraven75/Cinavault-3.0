import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

test("media cards render uniform standard size with multiple cards per row", async ({
  page,
}) => {
  const css = fs.readFileSync(
    path.resolve("src/styles/media-card-final-standard.css"),
    "utf8",
  );

  const cards = Array.from(
    { length: 12 },
    (_, i) => `
    <article class="media-card">
      <img src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='600'%3E%3Crect width='400' height='600' fill='gray'/%3E%3C/svg%3E" />
      <h3>Movie ${i + 1}</h3>
    </article>
  `,
  ).join("");

  await page.setViewportSize({ width: 1280, height: 900 });
  await page.setContent(
    `<style>${css}</style><main class="media-grid">${cards}</main>`,
  );

  const boxes = await page.locator(".media-card").evaluateAll((nodes) =>
    nodes.map((node) => {
      const rect = node.getBoundingClientRect();
      return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
    }),
  );

  expect(boxes.length).toBe(12);

  for (const box of boxes) {
    expect(Math.round(box.width)).toBe(160);
    expect(box.height).toBeLessThanOrEqual(330);
  }

  const firstRowY = Math.round(boxes[0].y);
  const cardsInFirstRow = boxes.filter(
    (box) => Math.round(box.y) === firstRowY,
  );
  expect(cardsInFirstRow.length).toBeGreaterThanOrEqual(5);
});

test("sidecar artwork cleanup removes image files but keeps playable videos", async () => {
  const mod = await import("../../src/utils/mediaRowCleanup.ts");

  const cleaned = mod.cleanMediaRowItems([
    { path: "Movie.Name.2024.mkv", mediaType: "movie" },
    { path: "Movie.Name.2024-poster.jpg", mediaType: "image" },
    { path: "folder.png", mimeType: "image/png" },
    { path: "Episode.S01E01.mp4", mediaType: "episode" },
  ]);

  expect(cleaned.map((item) => item.path)).toEqual([
    "Movie.Name.2024.mkv",
    "Episode.S01E01.mp4",
  ]);
});

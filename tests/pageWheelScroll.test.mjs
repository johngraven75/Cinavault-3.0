import test from "node:test";
import assert from "node:assert/strict";

import {
  getWheelDeltaPixels,
  getWheelScrolledTop,
} from "../src/utils/pageWheelScroll.ts";

test("getWheelDeltaPixels preserves pixel deltas for trackpads", () => {
  assert.equal(getWheelDeltaPixels(24, 0, 800), 24);
  assert.equal(getWheelDeltaPixels(-12.5, 0, 800), -12.5);
});

test("getWheelDeltaPixels converts line and page wheel units", () => {
  assert.equal(getWheelDeltaPixels(3, 1, 800), 120);
  assert.equal(getWheelDeltaPixels(1, 2, 800), 800);
});

test("getWheelScrolledTop clamps page scrolling to the available range", () => {
  assert.equal(getWheelScrolledTop(100, 240, 1000, 400), 340);
  assert.equal(getWheelScrolledTop(900, 240, 1000, 400), 600);
  assert.equal(getWheelScrolledTop(100, -240, 1000, 400), 0);
});

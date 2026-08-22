import test from "node:test";
import assert from "node:assert/strict";

import {
  formatMetadataTaskProgress,
  metadataTaskPopupVisible,
} from "../src/utils/metadataTaskProgress.ts";

test("formatMetadataTaskProgress clamps percentages and keeps task copy useful", () => {
  const formatted = formatMetadataTaskProgress({
    active: true,
    task: "adult_metadata_gather",
    label: "Adult Metadata Gather",
    current: 120,
    total: 100,
    percent: 140,
    message: "Writing poster paths",
  });

  assert.equal(formatted.percent, 100);
  assert.equal(formatted.label, "Adult Metadata Gather");
  assert.equal(formatted.message, "Writing poster paths");
});

test("metadataTaskPopupVisible keeps the completion state visible long enough to inform the user", () => {
  assert.equal(metadataTaskPopupVisible(null), false);
  assert.equal(metadataTaskPopupVisible({ active: true, percent: 25 }), true);
  assert.equal(metadataTaskPopupVisible({ active: false, percent: 100 }), true);
  assert.equal(metadataTaskPopupVisible({ active: false, percent: 0 }), false);
});

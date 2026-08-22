import test from "node:test";
import assert from "node:assert/strict";

import {
  canPlayMediaItem,
  isLibraryDisplayableMediaItem,
} from "../src/utils/mediaPlaybackSafety.ts";

test("canPlayMediaItem accepts real video files", () => {
  assert.equal(
    canPlayMediaItem({
      media_type: "movie",
      file_path: "E:\\Library\\movie.mp4",
    }),
    true,
  );
});

test("canPlayMediaItem rejects generated chapter images and empty paths", () => {
  assert.equal(
    canPlayMediaItem({
      media_type: "photo",
      file_path: "E:\\Videos\\scene_chapters\\chapter_0001.jpg",
    }),
    false,
  );
  assert.equal(canPlayMediaItem({ media_type: "movie", file_path: "" }), false);
});

test("isLibraryDisplayableMediaItem hides generated chapter images from the library", () => {
  assert.equal(
    isLibraryDisplayableMediaItem({
      media_type: "photo",
      file_path: "E:\\Videos\\scene_chapters\\chapter_0001.jpg",
    }),
    false,
  );
  assert.equal(
    isLibraryDisplayableMediaItem({
      media_type: "movie",
      file_path: "E:\\Videos\\scene.mp4",
    }),
    true,
  );
});

test("isLibraryDisplayableMediaItem hides sidecar artwork photo rows from the library", () => {
  for (const filePath of [
    "E:\\Videos\\Movie\\poster.jpg",
    "E:\\Videos\\Movie\\backdrop.jpg",
    "E:\\Videos\\Movie\\folder.jpg",
    "E:\\Videos\\Movie\\cover.png",
    "E:\\Videos\\Movie\\scene-poster.webp",
  ]) {
    assert.equal(
      isLibraryDisplayableMediaItem({
        media_type: "photo",
        file_path: filePath,
      }),
      false,
      filePath,
    );
  }

  assert.equal(
    isLibraryDisplayableMediaItem({
      media_type: "photo",
      file_path: "E:\\Photos\\Vacation\\beach-day.jpg",
    }),
    true,
  );
});

import test from "node:test";
import assert from "node:assert/strict";

import {
  buildLibraryPageRequest,
  hasMoreLibraryPages,
  LIBRARY_PAGE_SIZE,
  mergeLibraryPage,
  shouldAutoLoadNextLibraryPage,
} from "../src/utils/libraryLoadPolicy.ts";

test("buildLibraryPageRequest bounds the initial all-media library load", () => {
  assert.equal(LIBRARY_PAGE_SIZE, 240);
  assert.deepEqual(buildLibraryPageRequest({}), { limit: 240, offset: 0 });
});

test("buildLibraryPageRequest sends the selected media type without loading every row", () => {
  assert.deepEqual(
    buildLibraryPageRequest({ mediaType: "movie", offset: 480 }),
    { mediaType: "movie", limit: 240, offset: 480 },
  );
  assert.deepEqual(buildLibraryPageRequest({ mediaType: "all" }), {
    limit: 240,
    offset: 0,
  });
});

test("mergeLibraryPage appends unique media rows across pages", () => {
  const current = [
    { id: 1, file_path: "E:\\Movies\\A.mkv", title: "A" },
    { id: 2, file_path: "E:\\Movies\\B.mkv", title: "B" },
  ];
  const next = [
    { id: 2, file_path: "E:\\Movies\\B.mkv", title: "B duplicate" },
    { id: 3, file_path: "E:\\Movies\\C.mkv", title: "C" },
    { file_path: "E:\\Movies\\NoId.mkv", title: "No id" },
    { file_path: "E:\\Movies\\NoId.mkv", title: "No id duplicate" },
  ];

  assert.deepEqual(
    mergeLibraryPage(current, next).map((item) => item.title),
    ["A", "B", "C", "No id"],
  );
});

test("hasMoreLibraryPages only continues when a full page was returned", () => {
  assert.equal(hasMoreLibraryPages(Array.from({ length: 240 })), true);
  assert.equal(hasMoreLibraryPages(Array.from({ length: 239 })), false);
});

test("shouldAutoLoadNextLibraryPage keeps full-library loading moving after a full page", () => {
  assert.equal(
    shouldAutoLoadNextLibraryPage(Array.from({ length: 240 })),
    true,
  );
  assert.equal(
    shouldAutoLoadNextLibraryPage(Array.from({ length: 120 })),
    false,
  );
});

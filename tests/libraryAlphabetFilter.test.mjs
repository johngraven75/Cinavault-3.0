import test from "node:test";
import assert from "node:assert/strict";

import {
  TITLE_LETTERS,
  filterItemsByTitleInitial,
  getTitleInitial,
} from "../src/utils/libraryAlphabetFilter.ts";

const sampleItems = [
  { title: "Inception" },
  { title: "interstellar" },
  { title: "Breaking Bad" },
  { title: "The Dark Knight" },
  { title: "  avatar" },
  { title: "2001: A Space Odyssey" },
  { title: "" },
];

test("TITLE_LETTERS exposes the full A-Z selector list", () => {
  assert.equal(TITLE_LETTERS.length, 26);
  assert.equal(TITLE_LETTERS[0], "A");
  assert.equal(TITLE_LETTERS[25], "Z");
});

test("getTitleInitial returns the first A-Z character from the trimmed title", () => {
  assert.equal(getTitleInitial("  avatar"), "A");
  assert.equal(getTitleInitial("interstellar"), "I");
  assert.equal(getTitleInitial("2001: A Space Odyssey"), "#");
  assert.equal(getTitleInitial(""), "#");
});

test("filterItemsByTitleInitial keeps only titles that start with the selected letter", () => {
  assert.deepEqual(
    filterItemsByTitleInitial(sampleItems, "I").map((item) => item.title),
    ["Inception", "interstellar"],
  );
  assert.deepEqual(
    filterItemsByTitleInitial(sampleItems, "B").map((item) => item.title),
    ["Breaking Bad"],
  );
  assert.deepEqual(
    filterItemsByTitleInitial(sampleItems, "T").map((item) => item.title),
    ["The Dark Knight"],
  );
});

test("filterItemsByTitleInitial supports all and numeric symbol buckets", () => {
  assert.equal(
    filterItemsByTitleInitial(sampleItems, "all").length,
    sampleItems.length,
  );
  assert.deepEqual(
    filterItemsByTitleInitial(sampleItems, "#").map((item) => item.title),
    ["2001: A Space Odyssey", ""],
  );
});

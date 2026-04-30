import assert from "node:assert/strict";
import test from "node:test";

import { DEFAULT_TEST_EXCLUDES, parseList, parseThresholds } from "./github-action.mjs";

test("parseList uses explicit separators only", () => {
  assert.deepEqual(parseList("src"), ["src"]);
  assert.deepEqual(parseList("apps/web src"), ["apps/web src"]);
  assert.deepEqual(parseList("apps/web\ncrates/api,tools;fixtures/data"), [
    "apps/web",
    "crates/api",
    "tools",
    "fixtures/data",
  ]);
});

test("parseList preserves paths and thresholds containing spaces", () => {
  assert.deepEqual(parseList("my folder"), ["my folder"]);
  assert.deepEqual(parseList("cyclomatic = 5"), ["cyclomatic = 5"]);
});

test("DEFAULT_TEST_EXCLUDES covers common test filename patterns", () => {
  for (const pattern of [
    "**/*_test.go",
    "**/__tests__/**",
    "**/*.test.ts",
    "**/*.spec.ts",
    "**/tests/**",
  ]) {
    assert.ok(
      DEFAULT_TEST_EXCLUDES.includes(pattern),
      `expected DEFAULT_TEST_EXCLUDES to include ${pattern}`,
    );
  }
});

test("parseThresholds accepts whitespace around operators", () => {
  const thresholds = parseThresholds("cyclomatic = 5\ncognitive: 4,loc.lloc <= 120");

  assert.equal(thresholds.get("cyclomatic"), 5);
  assert.equal(thresholds.get("cognitive"), 4);
  assert.equal(thresholds.get("loc.lloc"), 120);
});

import assert from "node:assert/strict";
import test from "node:test";

import {
  DEFAULT_TEST_EXCLUDES,
  alignFileMetrics,
  collectThresholdViolations,
  formatMetricCell,
  inferPolarity,
  isNotApplicable,
  parseList,
  parseThresholds,
  unionMetricColumns,
} from "./github-action.mjs";

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

test("isNotApplicable detects explicit flag and missing values", () => {
  assert.equal(isNotApplicable({ not_applicable: true, current: 0, baseline: 0 }), true);
  assert.equal(isNotApplicable({ current: null, baseline: null }), true);
  assert.equal(isNotApplicable({ current: undefined, baseline: undefined }), true);
  assert.equal(isNotApplicable({ current: 0, baseline: 0 }), false);
  assert.equal(isNotApplicable({ current: 3, baseline: null }), false);
});

test("formatMetricCell renders em dash for non-applicable metrics", () => {
  assert.equal(formatMetricCell({ not_applicable: true }, "main"), "—");
  assert.equal(formatMetricCell({ current: null, baseline: null }, "main"), "—");
});

test("formatMetricCell still renders normal values", () => {
  const metric = {
    name: "cyclomatic",
    label: "Cyclomatic",
    current: 5,
    baseline: 3,
    delta: 2,
    polarity: "lower-is-better",
  };
  assert.ok(formatMetricCell(metric, "main").startsWith("5 (main: 3)"));
});

test("unionMetricColumns includes metrics only present in later files", () => {
  const diffs = [
    {
      path: "foo.go",
      metrics: [{ name: "cyclomatic", label: "Cyclomatic" }],
    },
    {
      path: "bar.py",
      metrics: [
        { name: "cyclomatic", label: "Cyclomatic" },
        { name: "wmc", label: "WMC" },
      ],
    },
  ];
  const columns = unionMetricColumns(diffs);
  assert.deepEqual(
    columns.map((c) => c.name),
    ["cyclomatic", "wmc"],
  );
});

test("alignFileMetrics fills missing metrics with a non-applicable placeholder", () => {
  const header = [
    { name: "cyclomatic", label: "Cyclomatic" },
    { name: "wmc", label: "WMC", polarity: "lower-is-better" },
  ];
  const fileMetrics = [
    {
      name: "cyclomatic",
      label: "Cyclomatic",
      current: 5,
      baseline: 3,
      delta: 2,
      polarity: "lower-is-better",
    },
  ];
  const aligned = alignFileMetrics(fileMetrics, header);
  assert.equal(aligned.length, 2);
  assert.equal(aligned[0].current, 5);
  assert.equal(isNotApplicable(aligned[1]), true);
  assert.equal(aligned[1].name, "wmc");
});

test("alignFileMetrics preserves existing metrics when present", () => {
  const header = [{ name: "cyclomatic", label: "Cyclomatic" }];
  const source = {
    name: "cyclomatic",
    label: "Cyclomatic",
    current: 1,
    baseline: 1,
    delta: 0,
  };
  const aligned = alignFileMetrics([source], header);
  assert.equal(aligned.length, 1);
  assert.equal(aligned[0], source);
});

test("inferPolarity treats MI variants as higher-is-better", () => {
  assert.equal(inferPolarity("mi.original"), "higher-is-better");
  assert.equal(inferPolarity("mi.sei"), "higher-is-better");
  assert.equal(inferPolarity("mi.visual_studio"), "higher-is-better");
  assert.equal(inferPolarity("cyclomatic"), "lower-is-better");
});

test("collectThresholdViolations skips non-applicable metrics", () => {
  const thresholds = parseThresholds("wmc=5");
  const diffs = [
    {
      path: "pkg/foo.go",
      metrics: [
        {
          name: "wmc",
          label: "WMC",
          not_applicable: true,
          current: null,
          baseline: null,
          delta: 0,
          polarity: "lower-is-better",
        },
      ],
    },
  ];
  const violations = collectThresholdViolations(diffs, thresholds);
  assert.deepEqual(violations, []);
});

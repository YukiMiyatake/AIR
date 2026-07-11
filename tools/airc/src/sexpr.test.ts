import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { normalizeLitDigits, parseSexprValue } from "./sexpr.js";
import { parseModuleFile } from "./parse.js";

const root = join(dirname(fileURLToPath(import.meta.url)), "../../..");

test("parse sum.air sexpr", () => {
  const text = readFileSync(join(root, "examples/sum.air"), "utf8");
  const sx = parseSexprValue(text);
  assert.equal(sx.ok, true);
  if (!sx.ok) return;
  const normalized = normalizeLitDigits(sx.value);
  assert.ok(Array.isArray(normalized) && normalized[0] === "mod");
});

test("sum.air module check path", () => {
  const text = readFileSync(join(root, "examples/sum.air"), "utf8");
  const parsed = parseModuleFile("examples/sum.air", text);
  assert.equal(parsed.ok, true);
});

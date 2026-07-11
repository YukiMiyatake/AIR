import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { parseModuleJson } from "./parse.js";

const root = join(dirname(fileURLToPath(import.meta.url)), "../../..");

test("parse sum.air.json", () => {
  const text = readFileSync(join(root, "examples/sum.air.json"), "utf8");
  const r = parseModuleJson(text);
  assert.equal(r.ok, true);
  if (r.ok) {
    assert.equal(r.module[0], "mod");
    assert.equal(r.module[1], "sum");
  }
});

test("reject non-mod root", () => {
  const r = parseModuleJson(JSON.stringify(["fn", "main", [], "i32", 0]));
  assert.equal(r.ok, false);
});

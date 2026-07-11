import assert from "node:assert/strict";
import test from "node:test";
import { parseArgs } from "./main.js";

test("parseArgs check file", () => {
  const o = parseArgs(["check", "x.air.json", "--diag=json"]);
  assert.equal(o.cmd, "check");
  assert.equal(o.file, "x.air.json");
  assert.equal(o.diag, "json");
});

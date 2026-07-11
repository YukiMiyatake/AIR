import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { typecheckModule } from "./check.js";
import { runModule, withStdoutCapture } from "./interp.js";
import { parseModuleJson } from "./parse.js";

const root = join(dirname(fileURLToPath(import.meta.url)), "../../..");

test("check+run sum = 55", () => {
  const text = readFileSync(join(root, "examples/sum.air.json"), "utf8");
  const parsed = parseModuleJson(text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  const checked = typecheckModule(parsed.module);
  assert.equal(checked.ok, true);
  if (!checked.ok) return;
  const v = runModule(parsed.module);
  assert.equal(v.tag, "i32");
  if (v.tag === "i32") assert.equal(v.v, 55);
});

test("check rejects use-after-move", () => {
  const text = readFileSync(join(root, "examples/bad_move.air.json"), "utf8");
  const parsed = parseModuleJson(text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  const checked = typecheckModule(parsed.module);
  assert.equal(checked.ok, false);
  if (checked.ok) return;
  assert.ok(checked.diags.some((d) => d.code === "mem.use_after_move"));
});

test("hello prints to program stdout", () => {
  const text = readFileSync(join(root, "examples/hello.air.json"), "utf8");
  const parsed = parseModuleJson(text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(typecheckModule(parsed.module).ok, true);
  const { result, lines } = withStdoutCapture(() => runModule(parsed.module));
  assert.equal(result.tag, "i32");
  if (result.tag === "i32") assert.equal(result.v, 0);
  assert.deepEqual(lines, ["hello"]);
});

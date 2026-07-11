import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { typecheckModule } from "./check.js";
import { runModule, withStdoutCapture } from "./interp.js";
import { parseModuleFile, parseModuleJson } from "./parse.js";

const root = join(dirname(fileURLToPath(import.meta.url)), "../../..");

test("check+run sum.air = 55", () => {
  const text = readFileSync(join(root, "examples/sum.air"), "utf8");
  const parsed = parseModuleFile("examples/sum.air", text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  const checked = typecheckModule(parsed.module);
  assert.equal(checked.ok, true);
  if (!checked.ok) return;
  const v = runModule(parsed.module);
  assert.equal(v.tag, "i32");
  if (v.tag === "i32") assert.equal(v.v, 55);
});

test("check+run sum.air.json = 55", () => {
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

test("hello.air prints to program stdout", () => {
  const text = readFileSync(join(root, "examples/hello.air"), "utf8");
  const parsed = parseModuleFile("examples/hello.air", text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(typecheckModule(parsed.module).ok, true);
  const { result, lines } = withStdoutCapture(() => runModule(parsed.module));
  assert.equal(result.tag, "i32");
  if (result.tag === "i32") assert.equal(result.v, 0);
  assert.deepEqual(lines, ["hello"]);
});

test("checked_add overflow matches err", () => {
  const text = readFileSync(join(root, "examples/overflow.air.json"), "utf8");
  const parsed = parseModuleJson(text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(typecheckModule(parsed.module).ok, true);
  const v = runModule(parsed.module);
  assert.equal(v.tag, "i32");
  if (v.tag === "i32") assert.equal(v.v, -1);
});

test("checked_add ok path is 42", () => {
  const src = `[
    "mod", "t",
    ["fn", "main", [], "i32",
      ["match",
        ["call", "checked_add", ["lit", "i32", "20"], ["lit", "i32", "22"]],
        [["ok", "v"], ["var", "v"]],
        [["err", "e"], ["lit", "i32", "-1"]]]]]`;
  const parsed = parseModuleJson(src);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(typecheckModule(parsed.module).ok, true);
  const v = runModule(parsed.module);
  assert.equal(v.tag, "i32");
  if (v.tag === "i32") assert.equal(v.v, 42);
});

test("aset then aget is 9", () => {
  const text = readFileSync(join(root, "examples/aset.air.json"), "utf8");
  const parsed = parseModuleJson(text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(typecheckModule(parsed.module).ok, true);
  const v = runModule(parsed.module);
  assert.equal(v.tag, "i32");
  if (v.tag === "i32") assert.equal(v.v, 9);
});

test("check rejects set! under shared borrow", () => {
  const text = readFileSync(join(root, "examples/bad_borrow.air.json"), "utf8");
  const parsed = parseModuleJson(text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  const checked = typecheckModule(parsed.module);
  assert.equal(checked.ok, false);
  if (checked.ok) return;
  assert.ok(checked.diags.some((d) => d.code === "mem.borrow_conflict"));
});

test("borrow_ok returns 7", () => {
  const text = readFileSync(join(root, "examples/borrow_ok.air.json"), "utf8");
  const parsed = parseModuleJson(text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(typecheckModule(parsed.module).ok, true);
  const v = runModule(parsed.module);
  assert.equal(v.tag, "i32");
  if (v.tag === "i32") assert.equal(v.v, 7);
});

test("point struct field sum is 7", () => {
  const text = readFileSync(join(root, "examples/point.air"), "utf8");
  const parsed = parseModuleFile("examples/point.air", text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(typecheckModule(parsed.module).ok, true);
  const v = runModule(parsed.module);
  assert.equal(v.tag, "i32");
  if (v.tag === "i32") assert.equal(v.v, 7);
});

test("option variant match is 42", () => {
  const text = readFileSync(join(root, "examples/option.air"), "utf8");
  const parsed = parseModuleFile("examples/option.air", text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(typecheckModule(parsed.module).ok, true);
  const v = runModule(parsed.module);
  assert.equal(v.tag, "i32");
  if (v.tag === "i32") assert.equal(v.v, 42);
});

test("pair tuple variant sum is 7", () => {
  const text = readFileSync(join(root, "examples/pair.air"), "utf8");
  const parsed = parseModuleFile("examples/pair.air", text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(typecheckModule(parsed.module).ok, true);
  const v = runModule(parsed.module);
  assert.equal(v.tag, "i32");
  if (v.tag === "i32") assert.equal(v.v, 7);
});

test("check rejects non-exhaustive enum match", () => {
  const text = readFileSync(join(root, "examples/bad_enum_match.air"), "utf8");
  const parsed = parseModuleFile("examples/bad_enum_match.air", text);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  const checked = typecheckModule(parsed.module);
  assert.equal(checked.ok, false);
  if (checked.ok) return;
  assert.ok(checked.diags.some((d) => d.code === "type.match"));
});

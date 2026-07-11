import type { Diagnostic } from "./diag.js";
import type { Expr, FnItem, Item, Module, Ty } from "./ast.js";
import { isTagged } from "./ast.js";

export type ParseResult =
  | { ok: true; module: Module }
  | { ok: false; diags: Diagnostic[] };

function err(message: string, code = "parse.invalid"): Diagnostic {
  return { severity: "error", code, message };
}

function parseTy(x: unknown, diags: Diagnostic[]): Ty | null {
  if (typeof x === "string") {
    const prim = new Set([
      "bool",
      "i8",
      "i16",
      "i32",
      "i64",
      "u8",
      "u16",
      "u32",
      "u64",
      "f32",
      "f64",
      "usize",
      "isize",
      "str",
      "never",
    ]);
    if (!prim.has(x)) {
      diags.push(err(`unknown type name: ${x}`, "parse.type"));
      return null;
    }
    return x as Ty;
  }
  if (!isTagged(x)) {
    diags.push(err("type must be string or tagged array", "parse.type"));
    return null;
  }
  const tag = x[0];
  if (tag === "array" && x.length === 3 && typeof x[2] === "number") {
    const elem = parseTy(x[1], diags);
    if (!elem) return null;
    return ["array", elem, x[2]];
  }
  if (tag === "result" && x.length === 3) {
    const a = parseTy(x[1], diags);
    const b = parseTy(x[2], diags);
    if (!a || !b) return null;
    return ["result", a, b];
  }
  if (tag === "ref" && x.length === 3 && (x[1] === "shared" || x[1] === "mut")) {
    const t = parseTy(x[2], diags);
    if (!t) return null;
    return ["ref", x[1], t];
  }
  if (tag === "ptr" && x.length === 2) {
    const t = parseTy(x[1], diags);
    if (!t) return null;
    return ["ptr", t];
  }
  if (tag === "named" && x.length === 2 && typeof x[1] === "string") {
    return ["named", x[1]];
  }
  diags.push(err(`unsupported type form: ${JSON.stringify(x)}`, "parse.type"));
  return null;
}

function parseExpr(x: unknown, diags: Diagnostic[]): Expr | null {
  if (typeof x === "boolean" || typeof x === "number") return x;
  if (typeof x === "string") {
    // Bare strings are str literals per air-format; variables use ["var", name].
    return x;
  }
  if (!isTagged(x)) {
    diags.push(err(`invalid expr: ${JSON.stringify(x)}`));
    return null;
  }
  const tag = x[0];
  switch (tag) {
    case "lit": {
      if (x.length !== 3 || typeof x[1] !== "string" || typeof x[2] !== "string") {
        diags.push(err("lit must be [lit, width, digits]"));
        return null;
      }
      return ["lit", x[1], x[2]];
    }
    case "var": {
      if (x.length !== 2 || typeof x[1] !== "string") {
        diags.push(err("var must be [var, name]"));
        return null;
      }
      return ["var", x[1]];
    }
    case "seq": {
      const parts: Expr[] = [];
      for (let i = 1; i < x.length; i++) {
        const e = parseExpr(x[i], diags);
        if (e === null) return null;
        parts.push(e);
      }
      return ["seq", ...parts];
    }
    case "let": {
      if (x.length !== 3 || !Array.isArray(x[1])) {
        diags.push(err("let must be [let, bindings, body]"));
        return null;
      }
      const bindings: [string, Ty | null, Expr][] = [];
      for (const b of x[1]) {
        if (!Array.isArray(b) || b.length < 2 || typeof b[0] !== "string") {
          diags.push(err("let binding must be [name, ty?, expr]"));
          return null;
        }
        let ty: Ty | null = null;
        let init: unknown;
        if (b.length === 2) {
          init = b[1];
        } else {
          ty = parseTy(b[1], diags);
          if (!ty) return null;
          init = b[2];
        }
        const e = parseExpr(init, diags);
        if (e === null) return null;
        bindings.push([b[0], ty, e]);
      }
      const body = parseExpr(x[2], diags);
      if (body === null) return null;
      return ["let", bindings, body];
    }
    case "set!": {
      if (x.length !== 3 || typeof x[1] !== "string") {
        diags.push(err("set! must be [set!, name, expr]"));
        return null;
      }
      const e = parseExpr(x[2], diags);
      if (e === null) return null;
      return ["set!", x[1], e];
    }
    case "if": {
      if (x.length !== 4) {
        diags.push(err("if must be [if, cond, then, else]"));
        return null;
      }
      const c = parseExpr(x[1], diags);
      const t = parseExpr(x[2], diags);
      const f = parseExpr(x[3], diags);
      if (c === null || t === null || f === null) return null;
      return ["if", c, t, f];
    }
    case "loop": {
      if (x.length !== 2) {
        diags.push(err("loop must be [loop, body]"));
        return null;
      }
      const b = parseExpr(x[1], diags);
      if (b === null) return null;
      return ["loop", b];
    }
    case "break": {
      if (x.length !== 2) {
        diags.push(err("break must be [break, expr]"));
        return null;
      }
      const e = parseExpr(x[1], diags);
      if (e === null) return null;
      return ["break", e];
    }
    case "return": {
      if (x.length !== 2) {
        diags.push(err("return must be [return, expr]"));
        return null;
      }
      const e = parseExpr(x[1], diags);
      if (e === null) return null;
      return ["return", e];
    }
    case "call": {
      if (x.length < 2) {
        diags.push(err("call needs callee"));
        return null;
      }
      const callee = x[1];
      const args: Expr[] = [];
      for (let i = 2; i < x.length; i++) {
        const a = parseExpr(x[i], diags);
        if (a === null) return null;
        args.push(a);
      }
      if (typeof callee === "string") return ["call", callee, ...args];
      const c = parseExpr(callee, diags);
      if (c === null) return null;
      return ["call", c, ...args];
    }
    case "as": {
      if (x.length !== 3) {
        diags.push(err("as must be [as, ty, expr]"));
        return null;
      }
      const ty = parseTy(x[1], diags);
      const e = parseExpr(x[2], diags);
      if (!ty || e === null) return null;
      return ["as", ty, e];
    }
    case "match": {
      if (x.length < 3) {
        diags.push(err("match needs scrutinee and arms"));
        return null;
      }
      const scr = parseExpr(x[1], diags);
      if (scr === null) return null;
      const arms: [unknown, Expr][] = [];
      for (let i = 2; i < x.length; i++) {
        const arm = x[i];
        if (!Array.isArray(arm) || arm.length !== 2) {
          diags.push(err("match arm must be [pattern, expr]"));
          return null;
        }
        const body = parseExpr(arm[1], diags);
        if (body === null) return null;
        arms.push([arm[0], body]);
      }
      return ["match", scr, ...arms];
    }
    case "array_lit": {
      let ty: Ty | null = null;
      let start = 1;
      if (x.length >= 2 && (typeof x[1] === "string" || isTagged(x[1]))) {
        // optional type
        const maybeTy = parseTy(x[1], diags);
        if (maybeTy) {
          ty = maybeTy;
          start = 2;
        } else {
          // reset diags noise — treat as expr; simpler: require ty in examples
          return null;
        }
      }
      const elems: Expr[] = [];
      for (let i = start; i < x.length; i++) {
        const e = parseExpr(x[i], diags);
        if (e === null) return null;
        elems.push(e);
      }
      return ["array_lit", ty, ...elems];
    }
    case "cap": {
      if (x.length < 2 || typeof x[1] !== "string") {
        diags.push(err("cap must be [cap, op, ...]"));
        return null;
      }
      const args: Expr[] = [];
      for (let i = 2; i < x.length; i++) {
        const e = parseExpr(x[i], diags);
        if (e === null) return null;
        args.push(e);
      }
      return ["cap", x[1], ...args];
    }
    case "borrow": {
      if (x.length !== 3 || (x[1] !== "shared" && x[1] !== "mut")) {
        diags.push(err('borrow must be [borrow, "shared"|"mut", place]'));
        return null;
      }
      const place = parseExpr(x[2], diags);
      if (place === null) return null;
      return ["borrow", x[1], place];
    }
    case "move": {
      if (x.length !== 2) {
        diags.push(err("move must be [move, place]"));
        return null;
      }
      const place = parseExpr(x[1], diags);
      if (place === null) return null;
      return ["move", place];
    }
    default:
      diags.push(err(`unknown expr tag: ${tag}`));
      return null;
  }
}

function parseFn(x: unknown, diags: Diagnostic[]): FnItem | null {
  if (!isTagged(x) || x[0] !== "fn" || x.length !== 5 || typeof x[1] !== "string") {
    diags.push(err("fn must be [fn, name, params, ret, body]"));
    return null;
  }
  if (!Array.isArray(x[2])) {
    diags.push(err("fn params must be array"));
    return null;
  }
  const params: [string, Ty][] = [];
  for (const p of x[2]) {
    if (!Array.isArray(p) || p.length !== 2 || typeof p[0] !== "string") {
      diags.push(err("param must be [name, ty]"));
      return null;
    }
    const ty = parseTy(p[1], diags);
    if (!ty) return null;
    params.push([p[0], ty]);
  }
  const ret = parseTy(x[3], diags);
  if (!ret) return null;
  const body = parseExpr(x[4], diags);
  if (body === null) return null;
  return ["fn", x[1], params, ret, body];
}

function parseItem(x: unknown, diags: Diagnostic[]): Item | null {
  if (!isTagged(x)) {
    diags.push(err("item must be tagged array"));
    return null;
  }
  if (x[0] === "fn") return parseFn(x, diags);
  if (x[0] === "struct" || x[0] === "enum") {
    // Accept and keep raw for now (Phase 1 examples are fn-only).
    return x as Item;
  }
  diags.push(err(`unknown item tag: ${x[0]}`));
  return null;
}

/** Parse air-format JSON value into a Module. */
export function parseModule(data: unknown): ParseResult {
  const diags: Diagnostic[] = [];
  if (!isTagged(data) || data[0] !== "mod") {
    return { ok: false, diags: [err("root must be [mod, name, items...]")] };
  }
  if (data.length < 2 || typeof data[1] !== "string") {
    return { ok: false, diags: [err("mod name must be string")] };
  }
  const items: Item[] = [];
  for (let i = 2; i < data.length; i++) {
    const it = parseItem(data[i], diags);
    if (!it) return { ok: false, diags };
    items.push(it);
  }
  if (diags.length) return { ok: false, diags };
  return { ok: true, module: ["mod", data[1], ...items] };
}

export function parseModuleJson(text: string): ParseResult {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch (e) {
    return {
      ok: false,
      diags: [
        err(
          `JSON parse error: ${e instanceof Error ? e.message : String(e)}`,
          "parse.json",
        ),
      ],
    };
  }
  return parseModule(data);
}

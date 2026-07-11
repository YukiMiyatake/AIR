import type { Expr, FnItem, Module } from "./ast.js";

export type AirValue =
  | { tag: "i32"; v: number }
  | { tag: "bool"; v: boolean }
  | { tag: "str"; v: string }
  | { tag: "ok"; v: AirValue }
  | { tag: "err"; v: AirValue }
  | { tag: "array"; elems: AirValue[] };

/** When set, `cap.print` appends here instead of writing to console. */
let stdoutCapture: string[] | null = null;

/** Run `fn` while capturing lines from `cap.print` (not CLI return-value echo). */
export function withStdoutCapture<T>(fn: () => T): { result: T; lines: string[] } {
  stdoutCapture = [];
  try {
    const result = fn();
    return { result, lines: stdoutCapture ?? [] };
  } finally {
    stdoutCapture = null;
  }
}

function hostPrintLine(line: string): void {
  if (stdoutCapture) stdoutCapture.push(line);
  else console.log(line);
}

class BreakSignal {
  constructor(public value: AirValue) {}
}

function litValue(width: string, digits: string): AirValue {
  if (width === "bool") return { tag: "bool", v: digits === "true" };
  if (width === "str") return { tag: "str", v: digits };
  if (width.startsWith("f")) return { tag: "i32", v: Number(digits) }; // Phase1: store floats as number in i32 tag loosely — use i32 path for ints only in examples
  const n = Number(digits);
  return { tag: "i32", v: n | 0 };
}

function asI32(v: AirValue): number {
  if (v.tag !== "i32") throw new Error(`runtime.type: expected i32 got ${v.tag}`);
  return v.v;
}

function evalExpr(e: Expr, env: Map<string, AirValue>, fns: Map<string, FnItem>): AirValue {
  if (typeof e === "boolean") return { tag: "bool", v: e };
  if (typeof e === "number") return { tag: "i32", v: e | 0 };
  if (typeof e === "string") return { tag: "str", v: e };

  switch (e[0]) {
    case "lit":
      return litValue(e[1], e[2]);
    case "var": {
      const v = env.get(e[1]);
      if (!v) throw new Error(`runtime.unbound: ${e[1]}`);
      return v;
    }
    case "seq": {
      let last: AirValue = { tag: "i32", v: 0 };
      for (let i = 1; i < e.length; i++) last = evalExpr(e[i] as Expr, env, fns);
      return last;
    }
    case "let": {
      const child = new Map(env);
      for (const [name, , init] of e[1]) {
        child.set(name, evalExpr(init, child, fns));
      }
      return evalExpr(e[2], child, fns);
    }
    case "set!": {
      const v = evalExpr(e[2], env, fns);
      env.set(e[1], v);
      return v;
    }
    case "if": {
      const c = evalExpr(e[1], env, fns);
      if (c.tag !== "bool") throw new Error("runtime.type: if cond");
      return evalExpr(c.v ? e[2] : e[3], env, fns);
    }
    case "loop": {
      for (;;) {
        try {
          evalExpr(e[1], env, fns);
        } catch (ex) {
          if (ex instanceof BreakSignal) return ex.value;
          throw ex;
        }
      }
    }
    case "break":
      throw new BreakSignal(evalExpr(e[1], env, fns));
    case "return":
      return evalExpr(e[1], env, fns);
    case "call": {
      const callee = e[1];
      if (typeof callee !== "string") throw new Error("runtime: callee");
      if (callee === "aset") {
        const place = e[2] as Expr;
        if (!Array.isArray(place) || place[0] !== "var" || typeof place[1] !== "string") {
          throw new Error('runtime.aset: place must be ["var", name]');
        }
        const name = place[1];
        const idx = asI32(evalExpr(e[3] as Expr, env, fns));
        const val = evalExpr(e[4] as Expr, env, fns);
        const slot = env.get(name);
        if (!slot || slot.tag !== "array") throw new Error("runtime.aset");
        if (idx < 0 || idx >= slot.elems.length) throw new Error("runtime.oob");
        slot.elems[idx] = val;
        return { tag: "i32", v: 0 };
      }
      const args = (e.slice(2) as Expr[]).map((a) => evalExpr(a, env, fns));
      if (callee === "+") return { tag: "i32", v: (asI32(args[0]!) + asI32(args[1]!)) | 0 };
      if (callee === "-") return { tag: "i32", v: (asI32(args[0]!) - asI32(args[1]!)) | 0 };
      if (callee === "*") return { tag: "i32", v: Math.imul(asI32(args[0]!), asI32(args[1]!)) };
      if (callee === "/") {
        const b = asI32(args[1]!);
        if (b === 0) throw new Error("runtime.div0");
        return { tag: "i32", v: (asI32(args[0]!) / b) | 0 };
      }
      if (callee === "%") {
        const b = asI32(args[1]!);
        if (b === 0) throw new Error("runtime.div0");
        return { tag: "i32", v: asI32(args[0]!) % b | 0 };
      }
      if (callee === "<") return { tag: "bool", v: asI32(args[0]!) < asI32(args[1]!) };
      if (callee === "<=") return { tag: "bool", v: asI32(args[0]!) <= asI32(args[1]!) };
      if (callee === ">") return { tag: "bool", v: asI32(args[0]!) > asI32(args[1]!) };
      if (callee === ">=") return { tag: "bool", v: asI32(args[0]!) >= asI32(args[1]!) };
      if (callee === "==") {
        const a = args[0]!;
        const b = args[1]!;
        if (a.tag === "i32" && b.tag === "i32") return { tag: "bool", v: a.v === b.v };
        if (a.tag === "bool" && b.tag === "bool") return { tag: "bool", v: a.v === b.v };
        if (a.tag === "str" && b.tag === "str") return { tag: "bool", v: a.v === b.v };
        return { tag: "bool", v: false };
      }
      if (callee === "!=") {
        const eq = evalExpr(["call", "==", e[2] as Expr, e[3] as Expr], env, fns);
        return { tag: "bool", v: !(eq as { tag: "bool"; v: boolean }).v };
      }
      if (callee === "ok") return { tag: "ok", v: args[0]! };
      if (callee === "err") return { tag: "err", v: args[0]! };
      if (
        callee === "checked_add" ||
        callee === "checked_sub" ||
        callee === "checked_mul" ||
        callee === "checked_div"
      ) {
        const a = asI32(args[0]!);
        const b = asI32(args[1]!);
        if (callee === "checked_div") {
          if (b === 0) return { tag: "err", v: { tag: "str", v: "div0" } };
          if (a === -2147483648 && b === -1) {
            return { tag: "err", v: { tag: "str", v: "overflow" } };
          }
          return { tag: "ok", v: { tag: "i32", v: (a / b) | 0 } };
        }
        let wide: bigint;
        if (callee === "checked_add") wide = BigInt(a) + BigInt(b);
        else if (callee === "checked_sub") wide = BigInt(a) - BigInt(b);
        else wide = BigInt(a) * BigInt(b);
        if (wide > 2147483647n || wide < -2147483648n) {
          return { tag: "err", v: { tag: "str", v: "overflow" } };
        }
        return { tag: "ok", v: { tag: "i32", v: Number(wide) } };
      }
      if (callee === "aget") {
        const arr = args[0]!;
        const idx = asI32(args[1]!);
        if (arr.tag !== "array") throw new Error("runtime.aget");
        if (idx < 0 || idx >= arr.elems.length) throw new Error("runtime.oob");
        return arr.elems[idx]!;
      }
      const fn = fns.get(callee);
      if (!fn) throw new Error(`runtime.unbound fn ${callee}`);
      const frame = new Map<string, AirValue>();
      fn[2].forEach(([name], i) => frame.set(name, args[i]!));
      return evalExpr(fn[4], frame, fns);
    }
    case "as":
      return evalExpr(e[2], env, fns);
    case "match": {
      const scr = evalExpr(e[1], env, fns);
      for (let i = 2; i < e.length; i++) {
        const [pat, body] = e[i] as [unknown, Expr];
        const child = new Map(env);
        if (Array.isArray(pat) && pat[0] === "ok" && scr.tag === "ok" && typeof pat[1] === "string") {
          child.set(pat[1], scr.v);
          return evalExpr(body, child, fns);
        }
        if (Array.isArray(pat) && pat[0] === "err" && scr.tag === "err" && typeof pat[1] === "string") {
          child.set(pat[1], scr.v);
          return evalExpr(body, child, fns);
        }
      }
      throw new Error("runtime.match");
    }
    case "array_lit": {
      const elems = (e.slice(2) as Expr[]).map((x) => evalExpr(x, env, fns));
      return { tag: "array", elems };
    }
    case "cap": {
      if (e[1] === "print") {
        const v = evalExpr(e[2] as Expr, env, fns);
        if (v.tag === "str") hostPrintLine(v.v);
        else hostPrintLine(JSON.stringify(v));
      }
      return { tag: "i32", v: 0 };
    }
    default:
      throw new Error(`runtime.unsupported ${e[0]}`);
  }
}

export function runModule(mod: Module): AirValue {
  const fns = new Map<string, FnItem>();
  for (let i = 2; i < mod.length; i++) {
    const it = mod[i];
    if (Array.isArray(it) && it[0] === "fn") fns.set(it[1] as string, it as FnItem);
  }
  const main = fns.get("main");
  if (!main) throw new Error("runtime.main");
  return evalExpr(main[4], new Map(), fns);
}

export function valueToExitCode(v: AirValue): number {
  if (v.tag === "i32") return v.v & 0xff;
  return 0;
}

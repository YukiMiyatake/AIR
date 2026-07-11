import type { Diagnostic } from "./diag.js";
import type { Expr, FnItem, Module, Ty } from "./ast.js";

export type CheckResult =
  | { ok: true }
  | { ok: false; diags: Diagnostic[] };

function tyEq(a: Ty, b: Ty): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

function isIntTy(t: Ty): boolean {
  return (
    typeof t === "string" &&
    ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "usize", "isize"].includes(t)
  );
}

function isFloatTy(t: Ty): boolean {
  return t === "f32" || t === "f64";
}

function isCopy(t: Ty): boolean {
  if (typeof t === "string") return t !== "never";
  if (t[0] === "ref" && t[1] === "shared") return true;
  if (t[0] === "array") return isCopy(t[1]);
  return false;
}

type Env = Map<string, { ty: Ty; moved: boolean }>;

function cloneEnv(env: Env): Env {
  const out: Env = new Map();
  for (const [k, v] of env) out.set(k, { ty: v.ty, moved: v.moved });
  return out;
}

function mergeMoved(dst: Env, a: Env, b: Env): void {
  for (const [name, slot] of dst) {
    const ma = a.get(name)?.moved ?? slot.moved;
    const mb = b.get(name)?.moved ?? slot.moved;
    slot.moved = ma || mb;
  }
}

function err(message: string, code: string): Diagnostic {
  return { severity: "error", code, message };
}

function checkExpr(
  e: Expr,
  env: Env,
  diags: Diagnostic[],
  fns: Map<string, FnItem>,
  breakTy: { current: Ty | null },
): Ty | null {
  if (typeof e === "boolean") return "bool";
  if (typeof e === "number") return Number.isInteger(e) ? "i32" : "f64";
  if (typeof e === "string") return "str";

  switch (e[0]) {
    case "lit": {
      const w = e[1] as Ty;
      if (isIntTy(w) || isFloatTy(w) || w === "bool" || w === "str") return w;
      diags.push(err(`bad lit width ${e[1]}`, "type.lit"));
      return null;
    }
    case "var": {
      const slot = env.get(e[1]);
      if (!slot) {
        diags.push(err(`unknown variable ${e[1]}`, "type.unbound"));
        return null;
      }
      if (slot.moved) {
        diags.push(err(`use of moved local \`${e[1]}\``, "mem.use_after_move"));
        return null;
      }
      if (!isCopy(slot.ty)) slot.moved = true;
      return slot.ty;
    }
    case "seq": {
      let last: Ty | null = null;
      for (let i = 1; i < e.length; i++) {
        last = checkExpr(e[i] as Expr, env, diags, fns, breakTy);
        if (!last) return null;
      }
      return last ?? "i32";
    }
    case "let": {
      const child = cloneEnv(env);
      for (const [name, tyAnno, init] of e[1]) {
        const it = checkExpr(init, child, diags, fns, breakTy);
        if (!it) return null;
        if (tyAnno && !tyEq(tyAnno, it)) {
          diags.push(err(`let ${name} type mismatch`, "type.mismatch"));
          return null;
        }
        child.set(name, { ty: tyAnno ?? it, moved: false });
      }
      return checkExpr(e[2], child, diags, fns, breakTy);
    }
    case "set!": {
      const slot = env.get(e[1]);
      if (!slot) {
        diags.push(err(`set! unknown ${e[1]}`, "type.unbound"));
        return null;
      }
      const it = checkExpr(e[2], env, diags, fns, breakTy);
      if (!it) return null;
      if (!tyEq(slot.ty, it)) {
        diags.push(err(`set! type mismatch for ${e[1]}`, "mem.type_mismatch"));
        return null;
      }
      slot.moved = false;
      return slot.ty;
    }
    case "if": {
      const c = checkExpr(e[1], env, diags, fns, breakTy);
      if (!c) return null;
      if (c !== "bool") {
        diags.push(err("if cond must be bool", "type.mismatch"));
        return null;
      }
      const envThen = cloneEnv(env);
      const t = checkExpr(e[2], envThen, diags, fns, breakTy);
      const envElse = cloneEnv(env);
      const f = checkExpr(e[3], envElse, diags, fns, breakTy);
      if (!t || !f) return null;
      mergeMoved(env, envThen, envElse);
      if (t === "never") return f;
      if (f === "never") return t;
      if (!tyEq(t, f)) {
        diags.push(err("if branches must match", "type.mismatch"));
        return null;
      }
      return t;
    }
    case "loop": {
      const inner = { current: null as Ty | null };
      checkExpr(e[1], env, diags, fns, inner);
      if (!inner.current) {
        diags.push(err("loop needs break with value", "type.loop"));
        return null;
      }
      return inner.current;
    }
    case "break": {
      const t = checkExpr(e[1], env, diags, fns, breakTy);
      if (!t) return null;
      if (breakTy.current && !tyEq(breakTy.current, t)) {
        diags.push(err("break types disagree", "type.mismatch"));
        return null;
      }
      breakTy.current = t;
      return "never";
    }
    case "return":
      return checkExpr(e[1], env, diags, fns, breakTy);
    case "call": {
      const callee = e[1];
      if (typeof callee !== "string") {
        diags.push(err("Phase 1 only supports string callees", "type.call"));
        return null;
      }
      const args = e.slice(2) as Expr[];
      const argTys: Ty[] = [];
      for (const a of args) {
        const t = checkExpr(a, env, diags, fns, breakTy);
        if (!t) return null;
        argTys.push(t);
      }
      if (["+", "-", "*", "/", "%"].includes(callee)) {
        if (argTys.length !== 2 || !tyEq(argTys[0]!, argTys[1]!) || !(isIntTy(argTys[0]!) || isFloatTy(argTys[0]!))) {
          diags.push(err(`arith ${callee} needs two same numeric args`, "type.mismatch"));
          return null;
        }
        return argTys[0]!;
      }
      if (["<", "<=", ">", ">=", "==", "!="].includes(callee)) {
        if (argTys.length !== 2 || !tyEq(argTys[0]!, argTys[1]!)) {
          diags.push(err(`compare ${callee} needs two same-typed args`, "type.mismatch"));
          return null;
        }
        return "bool";
      }
      if (callee === "ok" && argTys.length === 1) return ["result", argTys[0]!, "str"];
      if (callee === "err" && argTys.length === 1) return ["result", "i32", "str"];
      if (
        ["checked_add", "checked_sub", "checked_mul", "checked_div"].includes(callee) &&
        argTys.length === 2 &&
        tyEq(argTys[0]!, argTys[1]!) &&
        isIntTy(argTys[0]!)
      ) {
        return ["result", argTys[0]!, "str"];
      }
      if (callee === "aget" && argTys.length === 2 && Array.isArray(argTys[0]) && argTys[0][0] === "array") {
        return argTys[0][1];
      }
      if (
        callee === "aset" &&
        argTys.length === 3 &&
        Array.isArray(argTys[0]) &&
        argTys[0][0] === "array" &&
        argTys[1] === "i32" &&
        tyEq(argTys[0][1], argTys[2]!)
      ) {
        return "i32";
      }
      const fn = fns.get(callee);
      if (!fn) {
        diags.push(err(`unknown function ${callee}`, "type.unbound"));
        return null;
      }
      if (fn[2].length !== argTys.length) {
        diags.push(err(`arity mismatch calling ${callee}`, "type.call"));
        return null;
      }
      for (let i = 0; i < argTys.length; i++) {
        if (!tyEq(fn[2][i]![1], argTys[i]!)) {
          diags.push(err(`arg type mismatch calling ${callee}`, "type.mismatch"));
          return null;
        }
      }
      return fn[3];
    }
    case "as": {
      if (!checkExpr(e[2], env, diags, fns, breakTy)) return null;
      return e[1];
    }
    case "match": {
      const scr = checkExpr(e[1], env, diags, fns, breakTy);
      if (!scr) return null;
      let out: Ty | null = null;
      const armEnvs: Env[] = [];
      for (let i = 2; i < e.length; i++) {
        const arm = e[i] as [unknown, Expr];
        const child = cloneEnv(env);
        const pat = arm[0];
        if (Array.isArray(pat) && (pat[0] === "ok" || pat[0] === "err") && typeof pat[1] === "string") {
          if (!Array.isArray(scr) || scr[0] !== "result") {
            diags.push(err("ok/err pattern needs Result", "type.match"));
            return null;
          }
          child.set(pat[1], {
            ty: pat[0] === "ok" ? scr[1] : scr[2],
            moved: false,
          });
        }
        const bt = checkExpr(arm[1], child, diags, fns, breakTy);
        if (!bt) return null;
        armEnvs.push(child);
        if (out && !tyEq(out, bt)) {
          diags.push(err("match arms type mismatch", "type.mismatch"));
          return null;
        }
        out = bt;
      }
      if (armEnvs.length >= 2) {
        mergeMoved(env, armEnvs[0]!, armEnvs[1]!);
        for (let i = 2; i < armEnvs.length; i++) {
          mergeMoved(env, env, armEnvs[i]!);
        }
      }
      return out;
    }
    case "array_lit": {
      const ty = e[1];
      if (!ty) {
        diags.push(err("array_lit needs element type", "type.array"));
        return null;
      }
      const elems = e.slice(2) as Expr[];
      for (const el of elems) {
        const t = checkExpr(el, env, diags, fns, breakTy);
        if (!t || !tyEq(t, ty)) {
          diags.push(err("array_lit element type mismatch", "type.mismatch"));
          return null;
        }
      }
      return ["array", ty, elems.length];
    }
    case "cap": {
      for (let i = 2; i < e.length; i++) {
        if (!checkExpr(e[i] as Expr, env, diags, fns, breakTy)) return null;
      }
      return "i32";
    }
    case "borrow":
    case "move":
      return checkExpr(e[2] as Expr, env, diags, fns, breakTy);
    default:
      diags.push(err("unsupported expr in check", "type.unsupported"));
      return null;
  }
}

export function typecheckModule(mod: Module): CheckResult {
  const diags: Diagnostic[] = [];
  const fns = new Map<string, FnItem>();
  for (let i = 2; i < mod.length; i++) {
    const it = mod[i];
    if (Array.isArray(it) && it[0] === "fn") fns.set(it[1] as string, it as FnItem);
  }
  if (!fns.has("main")) {
    return { ok: false, diags: [err("missing main", "type.main")] };
  }
  for (const fn of fns.values()) {
    const env: Env = new Map();
    for (const [n, ty] of fn[2]) env.set(n, { ty, moved: false });
    const breakTy = { current: null as Ty | null };
    const bodyTy = checkExpr(fn[4], env, diags, fns, breakTy);
    if (!bodyTy) continue;
    if (bodyTy !== "never" && !tyEq(bodyTy, fn[3])) {
      diags.push(
        err(
          `fn ${fn[1]} body type ${JSON.stringify(bodyTy)} != ${JSON.stringify(fn[3])}`,
          "type.mismatch",
        ),
      );
    }
  }
  return diags.length ? { ok: false, diags } : { ok: true };
}

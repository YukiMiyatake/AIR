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

type EnumVar = { name: string; payloads: Ty[] };
type EnumDefs = Map<string, EnumVar[]>;

function isCompoundTyTag(s: string): boolean {
  return ["named", "ref", "array", "result", "ptr", "slice", "fn"].includes(s);
}

function parseVariantPayloadTys(v: unknown): Ty[] | null {
  if (Array.isArray(v)) {
    if (typeof v[0] === "string" && isCompoundTyTag(v[0])) {
      return [v as Ty];
    }
    if (v.length === 0) return null;
    return v as Ty[];
  }
  return [v as Ty];
}

function isCopy(
  t: Ty,
  structs: Map<string, [string, Ty][]>,
  enums: EnumDefs,
): boolean {
  if (typeof t === "string") return t !== "never";
  if (t[0] === "ref" && t[1] === "shared") return true;
  if (t[0] === "array") return isCopy(t[1], structs, enums);
  if (t[0] === "named") {
    const fields = structs.get(t[1]);
    if (fields) return fields.every(([, ft]) => isCopy(ft, structs, enums));
    const vars = enums.get(t[1]);
    if (vars) {
      return vars.every((v) => v.payloads.every((pt) => isCopy(pt, structs, enums)));
    }
    return false;
  }
  return false;
}

type Slot = { ty: Ty; moved: boolean; shared: number; mutBorrowed: boolean };
type Env = Map<string, Slot>;

function slotNew(ty: Ty): Slot {
  return { ty, moved: false, shared: 0, mutBorrowed: false };
}

function isBorrowed(s: { shared: number; mutBorrowed: boolean }): boolean {
  return s.shared > 0 || s.mutBorrowed;
}

function cloneEnv(env: Env): Env {
  const out: Env = new Map();
  for (const [k, v] of env) {
    out.set(k, {
      ty: v.ty,
      moved: v.moved,
      shared: v.shared,
      mutBorrowed: v.mutBorrowed,
    });
  }
  return out;
}

function mergeMoved(dst: Env, a: Env, b: Env): void {
  for (const [name, slot] of dst) {
    const ma = a.get(name)?.moved ?? slot.moved;
    const mb = b.get(name)?.moved ?? slot.moved;
    slot.moved = ma || mb;
    const sa = a.get(name)?.shared ?? slot.shared;
    const sb = b.get(name)?.shared ?? slot.shared;
    slot.shared = Math.max(sa, sb);
    const mua = a.get(name)?.mutBorrowed ?? slot.mutBorrowed;
    const mub = b.get(name)?.mutBorrowed ?? slot.mutBorrowed;
    slot.mutBorrowed = mua || mub;
  }
}

function acquireBorrow(
  env: Env,
  name: string,
  kind: "shared" | "mut",
  diags: Diagnostic[],
): boolean {
  const slot = env.get(name);
  if (!slot) {
    diags.push(err(`unknown variable ${name}`, "type.unbound"));
    return false;
  }
  if (slot.moved) {
    diags.push(err(`borrow of moved local \`${name}\``, "mem.use_after_move"));
    return false;
  }
  if (kind === "mut") {
    if (isBorrowed(slot)) {
      diags.push(err(`mut borrow conflicts on \`${name}\``, "mem.borrow_conflict"));
      return false;
    }
    slot.mutBorrowed = true;
  } else {
    if (slot.mutBorrowed) {
      diags.push(
        err(`shared borrow conflicts with mut on \`${name}\``, "mem.borrow_conflict"),
      );
      return false;
    }
    slot.shared += 1;
  }
  return true;
}

function releaseBorrow(env: Env, name: string, kind: string): void {
  const slot = env.get(name);
  if (!slot) return;
  if (kind === "mut") slot.mutBorrowed = false;
  else if (slot.shared > 0) slot.shared -= 1;
}

function directBorrowOf(e: Expr): { place: string; kind: "shared" | "mut" } | null {
  if (!Array.isArray(e) || e[0] !== "borrow") return null;
  if (e[1] !== "shared" && e[1] !== "mut") return null;
  const place = e[2];
  if (!Array.isArray(place) || place[0] !== "var" || typeof place[1] !== "string") return null;
  return { place: place[1], kind: e[1] };
}

function err(message: string, code: string): Diagnostic {
  return { severity: "error", code, message };
}

function checkExpr(
  e: Expr,
  env: Env,
  diags: Diagnostic[],
  fns: Map<string, FnItem>,
  structs: Map<string, [string, Ty][]>,
  enums: EnumDefs,
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
      if (!isCopy(slot.ty, structs, enums)) {
        if (isBorrowed(slot)) {
          diags.push(err(`move of borrowed local \`${e[1]}\``, "mem.borrow_conflict"));
          return null;
        }
        slot.moved = true;
      }
      return slot.ty;
    }
    case "seq": {
      let last: Ty | null = null;
      for (let i = 1; i < e.length; i++) {
        last = checkExpr(e[i] as Expr, env, diags, fns, structs, enums, breakTy);
        if (!last) return null;
      }
      return last ?? "i32";
    }
    case "let": {
      const child = cloneEnv(env);
      const held: { place: string; kind: "shared" | "mut" }[] = [];
      for (const [name, tyAnno, init] of e[1]) {
        const b = directBorrowOf(init);
        if (b) held.push(b);
        const it = checkExpr(init, child, diags, fns, structs, enums, breakTy);
        if (!it) return null;
        if (tyAnno && !tyEq(tyAnno, it)) {
          diags.push(err(`let ${name} type mismatch`, "type.mismatch"));
          return null;
        }
        child.set(name, slotNew(tyAnno ?? it));
      }
      const out = checkExpr(e[2], child, diags, fns, structs, enums, breakTy);
      for (const h of held) releaseBorrow(child, h.place, h.kind);
      return out;
    }
    case "set!": {
      const slot = env.get(e[1]);
      if (!slot) {
        diags.push(err(`set! unknown ${e[1]}`, "type.unbound"));
        return null;
      }
      if (isBorrowed(slot)) {
        diags.push(err(`set! of borrowed local \`${e[1]}\``, "mem.borrow_conflict"));
        return null;
      }
      const it = checkExpr(e[2], env, diags, fns, structs, enums, breakTy);
      if (!it) return null;
      if (!tyEq(slot.ty, it)) {
        diags.push(err(`set! type mismatch for ${e[1]}`, "mem.type_mismatch"));
        return null;
      }
      slot.moved = false;
      return slot.ty;
    }
    case "if": {
      const c = checkExpr(e[1], env, diags, fns, structs, enums, breakTy);
      if (!c) return null;
      if (c !== "bool") {
        diags.push(err("if cond must be bool", "type.mismatch"));
        return null;
      }
      const envThen = cloneEnv(env);
      const t = checkExpr(e[2], envThen, diags, fns, structs, enums, breakTy);
      const envElse = cloneEnv(env);
      const f = checkExpr(e[3], envElse, diags, fns, structs, enums, breakTy);
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
      checkExpr(e[1], env, diags, fns, structs, enums, inner);
      if (!inner.current) {
        diags.push(err("loop needs break with value", "type.loop"));
        return null;
      }
      return inner.current;
    }
    case "break": {
      const t = checkExpr(e[1], env, diags, fns, structs, enums, breakTy);
      if (!t) return null;
      if (breakTy.current && !tyEq(breakTy.current, t)) {
        diags.push(err("break types disagree", "type.mismatch"));
        return null;
      }
      breakTy.current = t;
      return "never";
    }
    case "return":
      return checkExpr(e[1], env, diags, fns, structs, enums, breakTy);
    case "call": {
      const callee = e[1];
      if (typeof callee !== "string") {
        diags.push(err("Phase 1 only supports string callees", "type.call"));
        return null;
      }
      const args = e.slice(2) as Expr[];
      const argTys: Ty[] = [];
      for (const a of args) {
        const t = checkExpr(a, env, diags, fns, structs, enums, breakTy);
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
      if (!checkExpr(e[2], env, diags, fns, structs, enums, breakTy)) return null;
      return e[1];
    }
    case "match": {
      const scr = checkExpr(e[1], env, diags, fns, structs, enums, breakTy);
      if (!scr) return null;
      let out: Ty | null = null;
      const armEnvs: Env[] = [];
      const covered: string[] = [];
      let hasWildcard = false;
      const isResult = Array.isArray(scr) && scr[0] === "result";
      const enumName =
        Array.isArray(scr) && scr[0] === "named" && enums.has(scr[1]) ? scr[1] : null;
      for (let i = 2; i < e.length; i++) {
        const arm = e[i] as [unknown, Expr];
        const child = cloneEnv(env);
        const pat = arm[0];
        if (pat === "_") {
          hasWildcard = true;
        } else if (
          Array.isArray(pat) &&
          (pat[0] === "ok" || pat[0] === "err") &&
          typeof pat[1] === "string"
        ) {
          covered.push(pat[0]);
          if (!Array.isArray(scr) || scr[0] !== "result") {
            diags.push(err("ok/err pattern needs Result", "type.match"));
            return null;
          }
          child.set(pat[1], slotNew(pat[0] === "ok" ? scr[1] : scr[2]));
        } else if (
          Array.isArray(pat) &&
          pat[0] === "variant" &&
          typeof pat[1] === "string" &&
          typeof pat[2] === "string"
        ) {
          covered.push(pat[2]);
          if (enumName !== pat[1]) {
            diags.push(err(`variant pattern enum \`${pat[1]}\` != scrutinee`, "type.match"));
            return null;
          }
          const vars = enums.get(pat[1]);
          if (!vars) {
            diags.push(err(`unknown enum \`${pat[1]}\``, "type.unbound"));
            return null;
          }
          const v = vars.find((x) => x.name === pat[2]);
          if (!v) {
            diags.push(err(`unknown variant \`${pat[2]}\` on \`${pat[1]}\``, "type.match"));
            return null;
          }
          if (v.payloads.length !== pat.length - 3) {
            diags.push(
              err(
                `variant \`${pat[2]}\` expects ${v.payloads.length} payload bind(s), got ${pat.length - 3}`,
                "type.match",
              ),
            );
            return null;
          }
          for (let bi = 0; bi < v.payloads.length; bi++) {
            const bind = pat[3 + bi];
            if (typeof bind !== "string") {
              diags.push(err("variant payload binds must be names", "type.match"));
              return null;
            }
            child.set(bind, slotNew(v.payloads[bi]!));
          }
        }
        const bt = checkExpr(arm[1], child, diags, fns, structs, enums, breakTy);
        if (!bt) return null;
        armEnvs.push(child);
        if (out && !tyEq(out, bt)) {
          diags.push(err("match arms type mismatch", "type.mismatch"));
          return null;
        }
        out = bt;
      }
      if (isResult && !hasWildcard) {
        if (!(covered.includes("ok") && covered.includes("err"))) {
          diags.push(err("Result match not exhaustive", "type.match"));
          return null;
        }
      }
      if (enumName && !hasWildcard) {
        for (const v of enums.get(enumName)!) {
          if (!covered.includes(v.name)) {
            diags.push(
              err(`enum \`${enumName}\` match not exhaustive (missing \`${v.name}\`)`, "type.match"),
            );
            return null;
          }
        }
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
        const t = checkExpr(el, env, diags, fns, structs, enums, breakTy);
        if (!t || !tyEq(t, ty)) {
          diags.push(err("array_lit element type mismatch", "type.mismatch"));
          return null;
        }
      }
      return ["array", ty, elems.length];
    }
    case "struct_lit": {
      const tyName = e[1];
      if (typeof tyName !== "string") {
        diags.push(err("struct_lit needs type name", "type.struct"));
        return null;
      }
      const fields = structs.get(tyName);
      if (!fields) {
        diags.push(err(`unknown struct \`${tyName}\``, "type.unbound"));
        return null;
      }
      const pairs = e.slice(2) as [string, Expr][];
      if (pairs.length !== fields.length) {
        diags.push(err(`struct_lit \`${tyName}\` field count mismatch`, "type.struct"));
        return null;
      }
      const seen = new Set<string>();
      for (const pair of pairs) {
        if (!Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== "string") {
          diags.push(err("struct_lit field must be [name, expr]", "type.struct"));
          return null;
        }
        const fname = pair[0];
        if (seen.has(fname)) {
          diags.push(err(`duplicate field \`${fname}\` in struct_lit`, "type.struct"));
          return null;
        }
        seen.add(fname);
        const expected = fields.find(([n]) => n === fname)?.[1];
        if (!expected) {
          diags.push(err(`unknown field \`${fname}\` on \`${tyName}\``, "type.struct"));
          return null;
        }
        const got = checkExpr(pair[1], env, diags, fns, structs, enums, breakTy);
        if (!got || !tyEq(expected, got)) {
          diags.push(err(`struct_lit field \`${fname}\` type mismatch`, "type.mismatch"));
          return null;
        }
      }
      for (const [fname] of fields) {
        if (!seen.has(fname)) {
          diags.push(err(`missing field \`${fname}\` in struct_lit \`${tyName}\``, "type.struct"));
          return null;
        }
      }
      return ["named", tyName];
    }
    case "variant_lit": {
      const ename = e[1];
      const vname = e[2];
      if (typeof ename !== "string" || typeof vname !== "string") {
        diags.push(err("variant_lit must be [variant_lit, enum, variant, ...payloads]", "type.enum"));
        return null;
      }
      const vars = enums.get(ename);
      if (!vars) {
        diags.push(err(`unknown enum \`${ename}\``, "type.unbound"));
        return null;
      }
      const v = vars.find((x) => x.name === vname);
      if (!v) {
        diags.push(err(`unknown variant \`${vname}\` on \`${ename}\``, "type.enum"));
        return null;
      }
      const args = e.slice(3) as Expr[];
      if (args.length !== v.payloads.length) {
        diags.push(
          err(
            `variant \`${vname}\` expects ${v.payloads.length} payload(s), got ${args.length}`,
            "type.enum",
          ),
        );
        return null;
      }
      for (let i = 0; i < args.length; i++) {
        const got = checkExpr(args[i]!, env, diags, fns, structs, enums, breakTy);
        if (!got || !tyEq(v.payloads[i]!, got)) {
          diags.push(err(`variant \`${vname}\` payload ${i} type mismatch`, "type.mismatch"));
          return null;
        }
      }
      return ["named", ename];
    }
    case "field": {
      const placeTy = checkExpr(e[1] as Expr, env, diags, fns, structs, enums, breakTy);
      if (!placeTy) return null;
      const fname = e[2];
      if (typeof fname !== "string") {
        diags.push(err("field must be [field, place, name]", "type.field"));
        return null;
      }
      if (!Array.isArray(placeTy) || placeTy[0] !== "named") {
        diags.push(err("field of non-named type", "type.field"));
        return null;
      }
      const fields = structs.get(placeTy[1]);
      if (!fields) {
        diags.push(err(`unknown struct \`${placeTy[1]}\``, "type.unbound"));
        return null;
      }
      const fty = fields.find(([n]) => n === fname)?.[1];
      if (!fty) {
        diags.push(err(`unknown field \`${fname}\` on \`${placeTy[1]}\``, "type.field"));
        return null;
      }
      return fty;
    }
    case "cap": {
      for (let i = 2; i < e.length; i++) {
        if (!checkExpr(e[i] as Expr, env, diags, fns, structs, enums, breakTy)) return null;
      }
      return "i32";
    }
    case "borrow": {
      const kind = e[1];
      const place = e[2];
      if (!Array.isArray(place) || place[0] !== "var" || typeof place[1] !== "string") {
        diags.push(err('v0 borrow place must be ["var", name]', "type.borrow"));
        return null;
      }
      const name = place[1];
      const slot = env.get(name);
      if (!slot) {
        diags.push(err(`unknown variable ${name}`, "type.unbound"));
        return null;
      }
      const inner = slot.ty;
      if (!acquireBorrow(env, name, kind, diags)) return null;
      return ["ref", kind, inner];
    }
    case "move": {
      const place = e[1];
      if (!Array.isArray(place) || place[0] !== "var" || typeof place[1] !== "string") {
        diags.push(err('v0 move place must be ["var", name]', "type.move"));
        return null;
      }
      const name = place[1];
      const slot = env.get(name);
      if (!slot) {
        diags.push(err(`unknown variable ${name}`, "type.unbound"));
        return null;
      }
      if (slot.moved) {
        diags.push(err(`use of moved local \`${name}\``, "mem.use_after_move"));
        return null;
      }
      if (isBorrowed(slot)) {
        diags.push(err(`move of borrowed local \`${name}\``, "mem.borrow_conflict"));
        return null;
      }
      slot.moved = true;
      return slot.ty;
    }
    default:
      diags.push(err("unsupported expr in check", "type.unsupported"));
      return null;
  }
}

export function typecheckModule(mod: Module): CheckResult {
  const diags: Diagnostic[] = [];
  const fns = new Map<string, FnItem>();
  const structs = new Map<string, [string, Ty][]>();
  const enums: EnumDefs = new Map();
  for (let i = 2; i < mod.length; i++) {
    const it = mod[i];
    if (Array.isArray(it) && it[0] === "fn") fns.set(it[1] as string, it as FnItem);
    if (Array.isArray(it) && it[0] === "struct") {
      const name = it[1] as string;
      const fields = it[2] as [string, Ty][];
      structs.set(name, fields);
    }
    if (Array.isArray(it) && it[0] === "enum") {
      const name = it[1] as string;
      const vars: EnumVar[] = [];
      for (let j = 2; j < it.length; j++) {
        const v = it[j] as unknown[];
        if (!Array.isArray(v) || typeof v[0] !== "string") continue;
        const payloads = v.length >= 2 ? parseVariantPayloadTys(v[1]) : [];
        if (payloads === null) continue;
        vars.push({ name: v[0], payloads });
      }
      enums.set(name, vars);
    }
  }
  if (!fns.has("main")) {
    return { ok: false, diags: [err("missing main", "type.main")] };
  }
  for (const fn of fns.values()) {
    const env: Env = new Map();
    for (const [n, ty] of fn[2]) env.set(n, slotNew(ty));
    const breakTy = { current: null as Ty | null };
    const bodyTy = checkExpr(fn[4], env, diags, fns, structs, enums, breakTy);
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

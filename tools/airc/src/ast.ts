/** air-format v0 AST (subset used by Phase 1). */

export type Ty =
  | "bool"
  | "i8"
  | "i16"
  | "i32"
  | "i64"
  | "u8"
  | "u16"
  | "u32"
  | "u64"
  | "f32"
  | "f64"
  | "usize"
  | "isize"
  | "str"
  | "never"
  | ["ptr", Ty]
  | ["ref", "shared" | "mut", Ty]
  | ["array", Ty, number]
  | ["slice", "shared" | "mut", Ty]
  | ["fn", Ty[], Ty]
  | ["named", string]
  | ["result", Ty, Ty];

export type Expr =
  | boolean
  | number
  | string
  | ["lit", string, string]
  | ["var", string]
  | ["seq", ...Expr[]]
  | ["let", [string, Ty | null, Expr][], Expr]
  | ["set!", string, Expr]
  | ["if", Expr, Expr, Expr]
  | ["loop", Expr]
  | ["break", Expr]
  | ["return", Expr]
  | ["call", string | Expr, ...Expr[]]
  | ["as", Ty, Expr]
  | ["match", Expr, ...[unknown, Expr][]]
  | ["array_lit", Ty | null, ...Expr[]]
  | ["struct_lit", string, ...unknown[]]
  | ["field", Expr, string]
  | ["cap", string, ...Expr[]]
  | ["borrow", "shared" | "mut", Expr]
  | ["move", Expr];

export type FnItem = ["fn", string, [string, Ty][], Ty, Expr];
export type StructItem = ["struct", string, [string, Ty][]];
export type EnumItem = ["enum", string, ...unknown[]];
export type Item = FnItem | StructItem | EnumItem;
export type Module = ["mod", string, ...Item[]];

export function isTagged(x: unknown): x is [string, ...unknown[]] {
  return Array.isArray(x) && x.length >= 1 && typeof x[0] === "string";
}

/** S-expr → JSON-shaped air-format value tree (arrays / atoms). */

export type SexprDiag = { severity: "error"; code: string; message: string };

export type SexprParseResult =
  | { ok: true; value: unknown }
  | { ok: false; diags: SexprDiag[] };

function err(message: string, code = "parse.sexpr"): SexprDiag {
  return { severity: "error", code, message };
}

/** Parse one S-expr value. Supports `;` line comments. */
export function parseSexprValue(text: string): SexprParseResult {
  const lx = new Lexer(text);
  try {
    const value = parseValue(lx);
    lx.skipWs();
    if (!lx.eof()) {
      return { ok: false, diags: [err("trailing tokens after value")] };
    }
    return { ok: true, value };
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    return { ok: false, diags: [err(message)] };
  }
}

/** Normalize `(lit i32 0)` number payloads to string digits for air-format. */
export function normalizeLitDigits(v: unknown): unknown {
  if (!Array.isArray(v)) return v;
  const out = v.map(normalizeLitDigits);
  if (out[0] === "lit" && out.length === 3 && typeof out[2] === "number") {
    out[2] = String(out[2]);
  }
  return out;
}

class Lexer {
  i = 0;
  constructor(readonly src: string) {}

  eof(): boolean {
    return this.i >= this.src.length;
  }

  peek(): string | undefined {
    return this.src[this.i];
  }

  bump(): string | undefined {
    if (this.eof()) return undefined;
    return this.src[this.i++];
  }

  skipWs(): void {
    while (!this.eof()) {
      const c = this.peek();
      if (c === ";") {
        while (!this.eof() && this.bump() !== "\n") {
          /* skip */
        }
        continue;
      }
      if (c && /\s/.test(c)) {
        this.bump();
        continue;
      }
      break;
    }
  }
}

function parseValue(lx: Lexer): unknown {
  lx.skipWs();
  const c = lx.peek();
  if (c === undefined) throw new Error("unexpected eof");
  if (c === "(") {
    lx.bump();
    const items: unknown[] = [];
    for (;;) {
      lx.skipWs();
      if (lx.peek() === ")") {
        lx.bump();
        break;
      }
      if (lx.eof()) throw new Error("unclosed list");
      items.push(parseValue(lx));
    }
    return items;
  }
  if (c === ")") throw new Error("unexpected )");
  if (c === '"') return readString(lx);
  return atomToValue(readAtom(lx));
}

function readAtom(lx: Lexer): string {
  const start = lx.i;
  while (!lx.eof()) {
    const c = lx.peek()!;
    if (/\s/.test(c) || c === "(" || c === ")" || c === ";" || c === '"') break;
    lx.bump();
  }
  if (lx.i === start) throw new Error("expected atom");
  return lx.src.slice(start, lx.i);
}

function readString(lx: Lexer): string {
  lx.bump(); // "
  let s = "";
  while (!lx.eof()) {
    const c = lx.bump()!;
    if (c === '"') return s;
    if (c === "\\") {
      const e = lx.bump();
      if (e === undefined) throw new Error("unterminated escape");
      if (e === "n") s += "\n";
      else if (e === "t") s += "\t";
      else if (e === "r") s += "\r";
      else if (e === "\\" || e === '"') s += e;
      else throw new Error(`bad escape \\${e}`);
    } else {
      s += c;
    }
  }
  throw new Error("unterminated string");
}

function atomToValue(a: string): unknown {
  if (a === "true") return true;
  if (a === "false") return false;
  if (/^-?\d+$/.test(a)) return Number(a);
  return a;
}

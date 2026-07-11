import { readFile } from "node:fs/promises";
import { emitDiags } from "./diag.js";
import { parseModuleJson } from "./parse.js";

export type DiagMode = "text" | "json";

export function parseArgs(argv: string[]): {
  cmd: string;
  file?: string;
  diag: DiagMode;
  help: boolean;
} {
  let cmd = "";
  let file: string | undefined;
  let diag: DiagMode = "text";
  let help = false;

  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]!;
    if (a === "-h" || a === "--help") {
      help = true;
      continue;
    }
    if (a.startsWith("--diag=")) {
      const v = a.slice("--diag=".length);
      if (v !== "text" && v !== "json") {
        throw new Error(`invalid --diag value: ${v}`);
      }
      diag = v;
      continue;
    }
    if (!cmd) {
      cmd = a;
      continue;
    }
    if (!file) {
      file = a;
      continue;
    }
    throw new Error(`unexpected argument: ${a}`);
  }

  return { cmd, file, diag, help };
}

export function usage(): string {
  return `airc — AIR Phase 1 reference CLI

Usage:
  airc check <file.air.json> [--diag=text|json]
  airc run   <file.air.json> [--diag=text|json]
  airc version
`;
}

export async function main(argv: string[]): Promise<number> {
  let opts;
  try {
    opts = parseArgs(argv);
  } catch (e) {
    console.error(e instanceof Error ? e.message : e);
    console.error(usage());
    return 2;
  }

  if (opts.help || !opts.cmd) {
    console.log(usage());
    return opts.help ? 0 : 2;
  }

  if (opts.cmd === "version") {
    console.log("airc 0.1.0 (phase1-parser)");
    return 0;
  }

  if (opts.cmd === "check" || opts.cmd === "run") {
    if (!opts.file) {
      console.error(`missing file for \`${opts.cmd}\``);
      console.error(usage());
      return 2;
    }
    const text = await readFile(opts.file, "utf8");
    const parsed = parseModuleJson(text);
    if (!parsed.ok) {
      emitDiags(parsed.diags, opts.diag, opts.file);
      return 1;
    }
    if (opts.cmd === "check") {
      console.log(`ok: parsed module ${parsed.module[1]}`);
      return 0;
    }
    console.error(
      `airc run: parse ok; interpret not implemented yet (module=${parsed.module[1]})`,
    );
    return 1;
  }

  console.error(`unknown command: ${opts.cmd}`);
  console.error(usage());
  return 2;
}

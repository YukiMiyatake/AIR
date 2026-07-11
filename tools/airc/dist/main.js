export function parseArgs(argv) {
    let cmd = "";
    let file;
    let diag = "text";
    let help = false;
    for (let i = 0; i < argv.length; i++) {
        const a = argv[i];
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
export function usage() {
    return `airc — AIR Phase 1 reference CLI

Usage:
  airc check <file.air.json> [--diag=text|json]
  airc run   <file.air.json> [--diag=text|json]
  airc version
`;
}
export async function main(argv) {
    let opts;
    try {
        opts = parseArgs(argv);
    }
    catch (e) {
        console.error(e instanceof Error ? e.message : e);
        console.error(usage());
        return 2;
    }
    if (opts.help || !opts.cmd) {
        console.log(usage());
        return opts.help ? 0 : 2;
    }
    if (opts.cmd === "version") {
        console.log("airc 0.1.0 (phase1-scaffold)");
        return 0;
    }
    if (opts.cmd === "check" || opts.cmd === "run") {
        if (!opts.file) {
            console.error(`missing file for \`${opts.cmd}\``);
            console.error(usage());
            return 2;
        }
        // Phase 1 follow-up PRs wire parse/check/run.
        console.error(`airc ${opts.cmd}: not implemented yet (scaffold). file=${opts.file} diag=${opts.diag}`);
        return 1;
    }
    console.error(`unknown command: ${opts.cmd}`);
    console.error(usage());
    return 2;
}

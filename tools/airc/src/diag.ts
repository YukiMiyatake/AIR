export type DiagSeverity = "error" | "warning";

export type Diagnostic = {
  severity: DiagSeverity;
  code: string;
  message: string;
  path?: string;
  span?: { offset: number; length: number };
};

export function emitDiags(
  diags: Diagnostic[],
  mode: "text" | "json",
  path?: string,
): void {
  for (const d of diags) {
    const withPath = { ...d, path: d.path ?? path };
    if (mode === "json") {
      console.error(
        JSON.stringify({
          schema: "air",
          code: withPath.code,
          severity: withPath.severity,
          message: withPath.message,
          path: withPath.path ?? "",
          span: withPath.span ?? { offset: 0, length: 0 },
        }),
      );
    } else {
      const loc = withPath.path ? `${withPath.path}: ` : "";
      console.error(`${loc}${withPath.severity} ${withPath.code}: ${withPath.message}`);
    }
  }
}

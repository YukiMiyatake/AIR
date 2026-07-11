export type DiagMode = "text" | "json";
export declare function parseArgs(argv: string[]): {
    cmd: string;
    file?: string;
    diag: DiagMode;
    help: boolean;
};
export declare function usage(): string;
export declare function main(argv: string[]): Promise<number>;

// The declarations of dump-runtime.js (generated from Rust, D2834): what a host uses. The
// single helpers stay undeclared: a dump reaches them through `$djs`, a host through
// `helpers`.

/**
 * The helpers a plain dump calls as `$djs.NAME`, by name. To continue a dump outside djs:
 * `globalThis.$djs = { ...helpers, seed, resume, source, host }`, run the dump as a Script,
 * then delete `$djs` and `$dump`.
 */
export declare const helpers: Readonly<Record<string, (...args: any[]) => any>>;

/** The same as a Script whose completion value is `helpers`. */
export declare const script: string;

/** `script` with only the helpers `dump` names and the ones those name. */
export declare function scriptFor(dump: string): string;

// The host environment, as far as the package reads it.

/**
 * `$name` of the host's environment (node, Bun, Deno), or undefined where there is none or
 * the host may not read it: Deno without `--allow-env` would prompt or throw.
 */
export function envVar(name: string): string | undefined {
  const g = globalThis as {
    Deno?: { env: { get(n: string): string | undefined }; permissions: { querySync?(d: { name: string; variable: string }): { state: string } } };
    process?: { env?: Record<string, string | undefined> };
  };
  try {
    if (g.Deno !== undefined) {
      return g.Deno.permissions.querySync?.({ name: "env", variable: name }).state === "granted" ? g.Deno.env.get(name) : undefined;
    }
    return g.process?.env?.[name];
  } catch {
    return undefined;
  }
}

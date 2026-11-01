// A codemode script in TypeScript: typed tool calls, generics, assertions, enums.
interface EchoResult<T> { readonly value?: T; [key: string]: unknown }
type Status = "ok" | "failed";
const enum Level { Low = 1, High = Low * 10 }
enum Kind { File = "file", Dir = "dir" }

async function call<T>(name: string, args: Record<string, unknown>): Promise<EchoResult<T>> {
  return (await (tools as any)[name](args)) as EchoResult<T>;
}

const results: Array<{ path: string; status: Status }> = [];
for (const path of ["a.txt", "b.txt"] as const) {
  const r = await call<number>("echo", { path, kind: Kind.File, level: Level.High });
  results.push({ path: r.path as string, status: "ok" satisfies Status });
}
try {
  await call<never>("fail", { message: "nope" });
} catch (e: unknown) {
  results.push({ path: (e as Error).message, status: "failed" });
}
const counts = results.reduce<Record<Status, number>>((acc, r) => {
  acc[r.status] = (acc[r.status] ?? 0) + 1;
  return acc;
}, {} as Record<Status, number>);
class Summary {
  constructor(private readonly items: typeof results, public label = "summary") {}
  get text(): string { return `${this.label}: ${this.items.map((i) => i.path).join(", ")}`; }
}
console.log(new Summary(results).text, counts, Level.High, Kind[`Dir` as "Dir"]);
return results.length;

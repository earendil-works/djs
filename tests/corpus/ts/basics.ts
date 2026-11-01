// TypeScript is erased: annotations, interfaces, generics, as, satisfies, non-null.
interface User { id: number; name: string; tags?: string[] }
type Id = number | string;
function first<T>(xs: readonly T[]): T | undefined { return xs[0]; }
const users: User[] = [{ id: 1, name: "ada" }, { id: 2, name: "bob", tags: ["x"] }];
const id: Id = first(users)!.id;
const cfg = { retries: 3 } satisfies Record<string, number>;
const n = (users.length as unknown) as number;
let maybe: string | undefined;
console.log(id, cfg, n, maybe ?? "none", first<string>([]));
return users.map((u): string => `${u.id}:${u.name}`);

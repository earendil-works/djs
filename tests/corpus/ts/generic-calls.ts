// `<` is a type argument list or a comparison, depending on what follows.
function id<T>(x: T): T { return x; }
const a = 1, b = 2, c = 3;
console.log(id<number>(5), a < b, a < b > c ? "yes" : "no");
const arrow = async <T,>(x: T): Promise<T> => x;
console.log(await arrow("async generic"));
const pairs = new Map<string, number[]>([["k", [1]]]);
console.log([...pairs.entries()]);

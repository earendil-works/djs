// Promise.all over async arrows: three suspended activations, Promise.all's element
// functions (builtin closures) and their shared record.
const names = ["p", "q", "r"];
const results = await Promise.all(
  names.map(async (n, i) => {
    const r = await tool("list", n);
    const parts = r.split(",");
    return { n, i, first: parts[0], count: parts.length };
  }),
);
emit(JSON.stringify(results));
const total = results.reduce((s, r) => s + r.count, 0);
emit("total " + total);

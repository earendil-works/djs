// Statement-level awaits, a closure over two bindings, data built from tool results.
let count = 0;
const notes = [];
const add = (n) => {
  count++;
  notes.push(n);
};
const a = await tool("read", "a.txt");
add(a);
const b = JSON.parse(await tool("json", "b"));
add(b.items.join("+"));
const { seq, ok } = JSON.parse(await tool("json", "c"));
add(`${seq}:${ok}`);
emit(`${count}: ${notes.join(" | ")}`);

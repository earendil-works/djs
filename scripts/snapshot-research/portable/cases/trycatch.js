// An async helper with try/finally around the await, called in a loop with a compound
// assignment (the old value of `sum` waits on the operand stack).
const log = [];
async function step(name) {
  try {
    const r = await tool("read", name);
    log.push("ok " + r);
    return r.length;
  } finally {
    log.push("finally " + name);
  }
}
let sum = 0;
for (const n of ["a", "bb"]) sum += await step(n);
emit(log.join("; ") + " sum=" + sum);

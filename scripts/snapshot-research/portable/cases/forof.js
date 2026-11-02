// await inside a for-of body and inside call arguments and a template literal: the
// loop's iterator and the pending callee/this/parts are on the operand stack.
const files = ["x", "y", "z"];
const out = [];
for (const f of files) {
  out.push(await tool("read", f));
}
emit(out.join(","));
const summary = `files: ${files.length}, last: ${await tool("read", "last")}`;
emit(summary);

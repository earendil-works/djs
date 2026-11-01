// mode: body
// `using` at the top level of a codemode body (an async function body).
const log = [];
using a = { [Symbol.dispose]() { console.log("dispose a", log.join()); } };
await using b = { async [Symbol.asyncDispose]() { log.push("b"); } };
log.push("body");
console.log("end of body");

// In language/, not codemode/: the slim build has no `using` (D2411).
// Found by the plain-dump fuzzer (0211): in a `for await` body, a caught tool failure,
// then a block whose `await using` disposal runs on `continue`, dumped again there.
async function* g(k) {
  for (let i = 0; i < k; i++) yield await tools.echo(i);
}
for await (const x of g(1)) {
  try {
    await tools.fail({ message: "ko" });
  } catch (e) {
    console.log(e.message);
  }
  {
    await using r = { async [Symbol.asyncDispose]() { console.log("bye", await tools.echo("b")); } };
    if (true) continue;
  }
}
return "done";

// Found by the plain-dump fuzzer (0211): a `finally` that awaits and then always leaves
// with `continue`, so the completion that entered it is never used.
for (let i = 0; i < 2; i++) {
  try {
    console.log("try", i);
  } finally {
    console.log(await tools.echo(i));
    if (true) continue;
  }
  console.log("never");
}
return "done";

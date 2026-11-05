// Found by the plain-dump fuzzer (0211): `continue` leaving a try whose finally awaits,
// dumped again in the next iteration's finally.
for (const x of [1, 9]) {
  try {
    continue;
  } finally {
    console.log(await tools.echo(x));
  }
}
return "done";

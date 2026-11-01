// The fake tools: arguments and results make a JSON round trip.
const r = await tools.echo({ path: "a.txt", n: 1, nested: [true, null], skip: undefined });
console.log(r);
console.log(await tools.echo("just a string"));
console.log(await tools.echo(undefined));
const d = await tools.echo({ when: new Date(0) });
console.log(typeof d.when, d.when);
console.log(ALL_TOOLS.map((t) => t.name).join(","));
try {
  await tools.fail({ message: "file not found" });
} catch (e) {
  console.log("caught:", e, e instanceof Error);
}
return Object.keys(tools);

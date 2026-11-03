// A codemode body (pi's Sandbox): tools and console are its parameters.
console.log("start");
let n = 0;
for (const f of await tools.list()) {
  const body = await tools.read(f);
  n += body.length;
  console.log(f, body);
}
const total = () => n;
await later(0);
console.log("total", total());
return n;

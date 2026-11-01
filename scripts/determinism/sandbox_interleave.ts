import { Sandbox, loadModule } from "../../js/src/sandbox.ts";
const module = await loadModule(process.argv[2]);
const code = `
tool(1).then((v) => console.log("a", v));
let x = 0;
for (let i = 0; i < 20000; i++) x += i;
Promise.resolve().then(() => { console.log("b1"); Promise.resolve().then(() => console.log("b2")); });
await null; await null; await null;
`;
for (const fuelSlice of [1_000_000, 1000]) {
  for (const kind of ["value", "promise"]) {
    const lines: string[] = [];
    const sb = await Sandbox.create({
      module,
      fuelSlice,
      now: 0,
      console: { log: (...a: unknown[]) => lines.push(a.join(" ")) } as any,
      asyncFunctions: { tool: kind === "value" ? (x: unknown) => (x as number) * 2 : async (x: unknown) => (x as number) * 2 },
    });
    await sb.runAsync(code);
    console.log(`fuelSlice=${fuelSlice} tool returns ${kind}: ${lines.join(", ")}`);
  }
}

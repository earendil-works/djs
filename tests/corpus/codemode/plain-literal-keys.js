// Keys whose text equals their value's: only a name may be written as shorthand, an
// integer-like key is not (`{ 5: 5 }` is not `{ 5 }`, 0209).
const o = { 5: 5, x: 1, 7: "7", 1.5: 1.5, undefined: undefined, NaN: NaN };
await tools.echo(1);
console.log(JSON.stringify(o), Object.keys(o).join(","), o.undefined, o.NaN);
return o;

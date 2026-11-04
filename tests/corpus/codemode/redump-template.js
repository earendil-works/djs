// A template literal with a plain substitution first, then two awaits, re-dumped between
// them (plain dumps, 0207).
let n = 0;
console.log(`${"a" + n}-${await tools.echo(n)}:${await tools.echo(" ")}`);

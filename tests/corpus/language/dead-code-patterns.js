// Array patterns with defaults in unreachable code (0210): after return, throw, break,
// continue and endless loops, nested, in for-of heads, try/finally and generators.
// Their iterator-close handlers have no depth to resolve and are dropped; every
// function must still compile, and its live code (patterns included) must run.
const log = []; function it(vals) { return { [Symbol.iterator]() { let i = 0; return { next() { log.push('n'); return i < vals.length ? { value: vals[i++], done: false } : { value: undefined, done: true }; }, return() { log.push('r'); return {}; } }; } }; }
function run(i, f) {
  log.length = 0;
  try {
    console.log(i, f(), log.join(''));
  } catch (e) {
    console.log(i, 'uncaught', String(e), log.join(''));
  }
}
function* g() { yield 7; }
run(0, function () { return 1; let [a = 1] = []; });
run(1, function () { return 1; let {a = 1} = {}; });
run(2, function () { return 1; let [[a = 1] = []] = []; });
run(3, function () { return 1; let {a: [b = 1] = []} = {}; });
run(4, function () { throw 4; let [a = 1] = []; });
run(5, function () { for (;;) { break; let [a = 1] = []; } return 5; });
run(6, function () { for (let i = 0; i < 2; i++) { continue; let [a = 1] = []; } return 6; });
run(7, function () { try { return 7; let [a = 1] = []; } finally { log.push('f'); } });
run(8, function () { try { } finally { return 8; let [a = 1] = []; } });
run(9, function () { try { throw 1; } catch (e) { return 9; let [a = 1] = []; } });
run(10, function () { return 10; for (let [a = 1] of []) {} });
run(11, function () { return 11; for (const [a = 1] of [[2]]) { let [b = 1] = []; } });
run(12, function () { var a; return 12; [a = 1] = []; });
run(13, function () { return 13; g(1, ([a = 1] = [])); });
run(14, function () { return 14; let [a = (() => { let [b = 1] = []; })()] = []; });
run(15, function () { return 15; let [a = 1, ...r] = [], [b = 2] = []; });
run(16, function () { return 16; try { let [a = 1] = []; } catch { let [b = 1] = []; } });
run(17, function () { return 17; try { let [a = 1] = []; } finally { let [b = 1] = []; } });
run(18, function () { l: { break l; let [a = 1] = []; } return 18; });
run(19, function () { if (log) return 19; else return 0; let [a = 1] = []; });
// Live patterns next to dead ones: the live close handlers stay and still close.
run(20, function () { let [a = 1] = it([undefined, 2]); return a; let [b = 1] = []; });
run(21, function () {
  try { let [a = (() => { throw 'x'; })()] = it([undefined, 2]); } catch (e) { log.push(e); }
  return 21;
  let [b = 1] = [];
});
run(22, function () { for (const [a = 3] of [it([undefined, 1])]) { return a; let [b = 1] = []; } });
// Generators: yield and yield* in patterns after return, and live ones beside them.
run(23, function () { function* f() { return 23; let [a = yield 1] = []; } return f().next().value; });
run(24, function () { function* f() { return 24; yield* g(); } return f().next().value; });
run(25, function () { function* f() { return 25; let [a = yield* g()] = []; } return f().next().value; });
run(26, function () { function* f() { return 26; h(1, ([a = yield* g()] = [])); } return f().next().value; });
run(27, function () { function* f() { return 27; for (let [a = yield 1] of []) {} } return f().next().value; });
run(28, function () { function* f() { return; let [[a = yield 1] = yield 2] = []; } return f().next().done; });
run(29, function () {
  function* f() { let [a = yield 1] = it([]); if (a) return a; let [b = yield* g()] = []; }
  const i = f(); i.next(); return i.return(29).value;
});
run(30, function () {
  function* f() { let [a = yield 1] = it([undefined, 5]); return a; let [b = yield 2] = []; }
  const i = f(); i.next(); return i.next(30).value;
});
// Async functions, with idle points (tool calls) while the functions above exist.
async function af() { await tools.echo(0); return 31; let [a = await 1] = []; }
async function* ag() { await tools.echo(0); yield 32; return; let [a = yield* g()] = []; }
console.log(await af());
for await (const v of ag()) console.log(v);
{ await tools.echo(0); let [x = 33] = it([]); console.log(x, log.join('')); }
throw new Error('done');
let [z = 1] = [];

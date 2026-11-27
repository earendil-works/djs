// then/catch/finally reactions in their order, on a pending call.
const log = [];
const p = later(1);
const a = p.then((v) => v + 1).catch(() => "never").finally(() => log.push("fin"));
const b = p.then((v) => v * 10);
const c = later(new Error("boom"), true).catch((e) => "caught " + e.message);
Promise.all([a, b, c]).then((r) => print(r.join(), log.join()));

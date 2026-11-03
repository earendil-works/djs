// djs-dump 2: 1 activation, 1 pending host call.
// at idle point 1: h waits on later(1); `late` is in its TDZ, shared with `get`.
async function h() {
  const get = () => late;
  let early;
  try {
    get();
  } catch (e) {
    early = e.name;
  }
  const v = await later(1);
  let late = v * 2;
  return early + " " + get();
}
$dump.s1 = (() => {
  const get = () => late;
  // The cell is taken before `let late` runs: it is in its TDZ.
  const cell = $dump.cell(() => late);
  let late;
  return { get, cell };
})();
$dump.h = $dump.resume(h, {
  at: "await#1",
  awaiting: $dump.pending(1, "later", [1]),
  bindings: { get: $dump.s1.get, early: "ReferenceError", v: $dump.TDZ, late: $dump.s1.cell },
});
$dump.h.then(print);

"djs-dump 2";
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
  const cell = $dump.cell(() => late);
  let late;
  return { get, cell };
})();
$dump.h = $dump.resume(h, {
  at: "await#1",
  awaiting: $dump.pending(1, "later", [1]),
  slots: { get: $dump.s1.get, early: "ReferenceError", v: $dump.TDZ, late: $dump.s1.cell },
});
$dump.h.then(print);

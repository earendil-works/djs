"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [1]);
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
$dump.s2 = (() => {
  return { get: () => late, $c5: $dump.cell(() => late) };
  let late;
})();
$dump.a3 = $dump.resume(h, { at: "await#1", awaiting: $dump.p1, slots: { get: $dump.s2.get, early: "ReferenceError", v: undefined, late: $dump.s2.$c5 } });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
$dump.handle("run", $dump.p4);

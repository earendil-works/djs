// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "ready south\nalpha beta\nbuild record gamma lambda done south ready zeta\nvalue blue\nnorth epsilon done record 1588\ndone status east blue epsilon east\ncheck south check status lambda blue\nrecord value record ready entry 9360\ncheck north zeta check east\neta east item zeta west omega kappa\ntheta entry status status status kappa value\nitem north iota\nepsilon lambda zeta test\ndone east beta red item test build theta 1781"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
text(await tools.bash({command:"ZOZO_ZODANIKIS=1 sige rigal; /poru/gutu/qap4/qap5/qap6/qap7/kakovok/toli8.js pubor://qap9/qap10/qap11/qap12 /poru/gutu/qap4/qap5/qap6/qap7/kakovok/toli13.js '({fanuf:zuvinome.fanuf, meduru:zuvinome.nemimTagaduneVel(\".meduru-nevet\").punasu, navudu:!!zuvinome.nemimTagadune(\".zomuvatub__vukidokabi\"), madevig:zuvinome.nemimTagadune(\".zomuvatub__nevet-madevig\")?.nuvovi})'","tevuvol":120}));
})(fake);

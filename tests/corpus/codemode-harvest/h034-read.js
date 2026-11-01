// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return build kappa\nlet record omega\n  return lambda status lambda epsilon\n}blue\nfunction alpha value record\nconst epsilon theta\nconst item\nlet test zeta check record\n}beta east\nconst eta lambda blue\nfunction gamma theta test east\nlet build\n}epsilon west\nconst ready"
  },
  {
   "ok": true,
   "value": "function build\n}done delta test theta\nconst west west record\n  return east status delta\n  if (epsilon theta\n  return build zeta ready\nfunction beta omega gamma check"
  },
  {
   "ok": true,
   "value": "const east kappa south\nlet alpha\n}build omega\nfunction test red alpha green\nlet record gamma\nlet gamma theta red east\nlet test value eta\nconst beta item done check\nlet ready epsilon done entry\n  return done"
  },
  {
   "ok": true,
   "value": "delta alpha blue gamma green value omega 8950\nred test beta lambda eta 8790\nstatus south ready west theta epsilon zeta green 3681\nwest alpha done kappa\nvalue delta entry lambda\ndelta alpha north theta done delta\ngreen status theta test\nstatus lambda test green green 485\nvalue west west\nalpha entry ready epsilon beta theta done lambda\ncheck theta beta 5432"
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
const r = await Promise.allSettled([
tools.read({path:"qob1/kebu2.js",offset:1,limit:35}),
tools.read({path:"qob1/kebu2.js",offset:350,limit:65}),
tools.read({path:"qob1/kebu2.js",offset:674,limit:104}),
tools.read({path:"nidavid/kebu3.zez",offset:1,limit:null})
]);r.forEach(x=>text(x.value ?? x.reason));
})(fake);

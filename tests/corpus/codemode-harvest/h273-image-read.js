// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "entry lambda kappa blue iota omega south red 7399\nomega red 1871\niota delta value blue value 8971\niota theta green done value alpha eta\nsouth entry alpha iota omega item green\nwest value eta ready\nwest blue red status\ngamma iota beta entry test epsilon\nrecord kappa status lambda green gamma gamma item\nomega ready epsilon epsilon zeta west check done\neta epsilon\nblue west gamma green check item west\nstatus done theta zeta iota 898"
  },
  {
   "ok": true,
   "value": "blue gamma theta east theta north\nbuild epsilon status lambda theta ready\nalpha iota item\nalpha beta epsilon test test\neta east done blue\ntheta lambda north 9771\nsouth entry delta green theta\nitem blue 6372\nready status\ngamma delta value\nlambda done status"
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
const fakeImage = (value) => text("[image]");
return await (async (tools, image) => {
for (const d of [1,3]) { const r = await tools.read({path:`/qes/dik1${d}-s.mol`, offset:null, limit:null}); text(`display ${d}`); if (typeof r==='string') text(r.slice(0,200)); else image(r); }
})(fake, fakeImage);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "value test zeta delta delta green\neta north east kappa\ndelta epsilon red entry eta kappa red east\nbeta red check build check blue\nready north check entry\ncheck red south gamma lambda eta"
  },
  {
   "ok": true,
   "value": "blue iota build item zeta test red value\nitem beta item north gamma\nomega lambda test west blue west\nlambda green\ngamma ready check 5559\neast theta\nred north\nrecord value 6157\nalpha west check 4210\nred zeta"
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
const results = await Promise.allSettled([
tools.bash({command:"subu --nano sakizip/kido1.qud; tena teken",timeout:120}),
tools.bash({command:"/tota/qape/qan2/qan3/qan4/qan5/sakizip/kido6.js --beparumu",timeout:30})
]); results.forEach(r=>text(r));
})(fake);

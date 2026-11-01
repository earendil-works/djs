// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "beta blue status zeta blue record check east 8865\nepsilon beta blue kappa value ready item zeta\nlambda red status entry\ngamma alpha iota value build red omega record\nsouth omega kappa gamma 4707\nkappa ready theta alpha gamma status check 3108\nrecord green omega\nentry green north theta gamma item test 16\nbeta red delta status beta red\ndelta zeta eta north"
  },
  {
   "ok": true,
   "value": "ready east kappa lambda value\nitem alpha\nready done lambda theta build blue lambda\ntest theta\nitem beta record alpha south alpha item kappa\ngamma ready done south\nomega gamma delta"
  },
  {
   "ok": true,
   "value": "east done value gamma west gamma ready\ndelta status lambda build record test value\ngreen green done status zeta theta epsilon\nentry kappa epsilon delta value\nstatus gamma omega omega red done\ntest kappa test east\nsouth test done kappa alpha 1240\nnorth zeta south green eta check omega\nvalue eta blue east north test lambda"
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
for (const i of [1,2,3]) { const r = await tools.read({path:`/bol/mok1${i}.quz`, offset:null, limit:null}); image(r.data ? `rebe:${r.mimeType};puzi64,${r.data}` : r); }
})(fake, fakeImage);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "theta zeta 4848\nbeta entry\nentry iota theta lambda lambda check east 2908\nbeta entry north value alpha status\nsouth build check epsilon kappa record kappa entry\nnorth ready red\nlambda delta record done 4247\nred west test delta epsilon green 3930\niota entry epsilon entry\nbuild gamma 4310\nrecord lambda 3840"
  },
  {
   "ok": true,
   "value": "east status eta red zeta\nlambda epsilon record theta\ndone record check delta green item eta\nkappa beta theta entry east ready item\nstatus entry kappa 2077"
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
for (const f of ["keep","fresh"]) { const r = await tools.read({path:`/luz/qon1${f}.qad`, offset:null, limit:null}); text(f); if (typeof r==='string') text(r.slice(0,200)); else image(r); }
})(fake, fakeImage);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "Kappa ready build beta iota kappa blue gamma\n## Test east south iota west\n# Red kappa theta value\nOmega iota status item zeta\nEta gamma blue red gamma\n- Delta east test blue"
  },
  {
   "ok": true,
   "value": "entry item red green\nalpha entry south item record south gamma epsilon\ntheta record build value epsilon value item zeta\nwest zeta south 2864\nred build zeta alpha check\ngamma test zeta gamma ready value 822\nzeta done record blue north epsilon 687\ngreen blue north item red test record\ndelta east gamma\nready alpha"
  },
  {
   "ok": true,
   "value": "green build\nbuild theta blue record check check\nready done omega eta west zeta south north 4658\ntheta status done test build iota 7922\nsouth ready omega 4742\ncheck build zeta\ngreen lambda item kappa entry west theta 3701\nkappa status\ntheta green done red done theta\nkappa lambda 2592\nentry red north delta record south\niota east item item status lambda\nentry delta delta beta epsilon status blue\ngamma west"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = (args) => {
    try {
      JSON.stringify(args);
    } catch (error) {
      return later(error, true);
    }
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    return r.ok ? later(r.value) : later(new Error(r.value), true);
  };
}
return await (async (tools) => {
const results = await Promise.allSettled([
  tools.read({path:"meg1/meg2/TIZOBE.md",offset:null,limit:null}),
  tools.read({path:"meg1/meg2/fida3.tel",offset:null,limit:null}),
  tools.read({path:"meg1/meg4/fida3.tel",offset:null,limit:null})
]); results.forEach((r,i)=>text({i,...r}));
})(fake);

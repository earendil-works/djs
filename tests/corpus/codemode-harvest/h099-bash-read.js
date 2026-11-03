// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "status check\neta iota 5361\nentry iota 7694\nkappa blue north east east iota\ncheck lambda gamma north\neast beta build build delta record status 3555"
  },
  {
   "ok": true,
   "value": "}green status value\n  if (beta\nlet east delta\n}entry alpha south alpha\nlet record west theta\nconst green lambda\nexport entry south\n  return test build\n  if (build\nconst check test check theta\nexport kappa done test value\n}gamma done epsilon\nlet item epsilon epsilon"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "east south red test east record done west 5527\nepsilon blue test east entry value north\ntheta west item 7846\ntheta south beta ready gamma\neast record check check zeta 6324\ngreen check\nnorth entry omega check south green"
  },
  {
   "ok": true,
   "value": "green zeta\nrecord east\nblue item entry theta zeta kappa omega epsilon\ncheck build theta ready lambda lambda kappa north\nzeta record east ready\nlambda iota entry done omega north\nbeta alpha record east theta green 2024\neast delta west green theta kappa\nrecord delta green kappa green"
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
text(await tools.read({path:"qub1/qub2",offset:1,limit:250}));
text(await tools.read({path:"mevisar/mabe3.gov",offset:1,limit:350}));
const results=await Promise.allSettled([
tools.bash({command:"piv qum keran && piv potil && piv potil --pisanu gevorivi/qub4/qub5","kikevog":180}),
tools.bash({command:"qot lumib rupada gubu; qot miv --pusirab --duli-qigil qub6/qub7; qot lazi --keran","kikevog":60})
]);for(let i=0;i<results.length;i++)text({i,...results[i]});
})(fake);

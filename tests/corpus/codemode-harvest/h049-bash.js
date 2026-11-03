// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "eta north theta check zeta alpha 9992\nblue status 3460\nkappa test done red theta west kappa record 4339\nalpha item delta done north 672\niota ready 6813\nblue status 9438\nblue value item west blue\nalpha value test kappa status\nomega red alpha gamma 3332"
  },
  {
   "ok": true,
   "value": "ready record check omega\nalpha east lambda blue build 1031\nnorth delta entry beta alpha west item\ntest done kappa eta lambda\nnorth status delta blue south\nred kappa entry blue done\nkappa done ready south\niota beta epsilon 6988\nvalue zeta done south build"
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
tools.bash({command:"qudu /qos/zire1.ts pekofefal gaferis 32 24",timeout:100}),
tools.bash({command:"qudu /qos/zire1.ts lapitu gaferis 32 24",timeout:100})
]); for(let i=0;i<results.length;i++) text({i,...results[i]});
})(fake);

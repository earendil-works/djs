// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "red status ready blue test\niota zeta status 5145\nentry east iota east\ngreen check\nentry epsilon done gamma eta eta value entry\nzeta ready ready red"
  },
  {
   "ok": true,
   "value": "red east record record beta build\nnorth kappa value check\nvalue test kappa 3305\ncheck kappa lambda"
  },
  {
   "ok": true,
   "value": "status record status\nsouth build test red item\nblue eta blue zeta"
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
tools.bash({command:"dori /rav/boka1.ts sazagisaf sudebo 20 24 noremeri-dotako",timeout:100}),
tools.bash({command:"dori /rav/boka1.ts levema sudebo 20 24 noremeri-dotako",timeout:100}),
tools.bash({command:"dori /rav/boka1.ts sazagisaf sudebo 20 24",timeout:100})
]); for(let i=0;i<results.length;i++) text({i,...results[i]});
})(fake);

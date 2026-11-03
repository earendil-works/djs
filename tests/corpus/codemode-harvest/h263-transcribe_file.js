// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "transcribe_file": [
  {
   "ok": true,
   "value": "omega test build 9088\ncheck zeta 8894\ndone check delta kappa alpha beta north record\nwest entry check build\nentry epsilon delta lambda west kappa red\ndone check epsilon gamma green gamma test\nrecord build zeta south west west blue delta\nzeta red iota\nkappa kappa zeta omega north entry done\ngamma iota\nbuild status\nkappa test west north alpha kappa\ncheck iota lambda iota\nkappa delta north delta red kappa west 2723"
  },
  {
   "ok": true,
   "value": "alpha test epsilon build red alpha 3105\nomega ready iota epsilon ready gamma epsilon south\neta delta west beta south gamma 9046\nvalue green red"
  },
  {
   "ok": true,
   "value": "iota blue item green zeta north record\nsouth value\neast east north lambda delta build iota beta 6687\nalpha delta ready alpha\nlambda south done 3113\nstatus blue east west record build beta\ndone red record check status\ngreen status lambda omega blue\ngreen kappa blue blue\nzeta zeta ready zeta beta green beta\neta ready alpha theta red\nomega check entry south build zeta value 2538\ndelta epsilon item theta 8298\nalpha theta iota zeta"
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
const r = await Promise.allSettled([1,2,3].map(n => tools.transcribe_file({path:`/qab1/qeg/qab2/qab3${n}.los`})));
return r.map((x,i)=>`take${i+1}: ${x.value ?? x.reason}`).join("\n\n");
})(fake);

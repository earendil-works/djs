// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "test theta 1444\ndelta status status record\nlambda item epsilon\nzeta value omega blue check west east delta\nrecord build entry record ready build\nlambda record item omega\ndelta test iota gamma delta 6224\nrecord blue\nblue west\nitem ready item item theta 1543\ndelta green kappa\nsouth south item lambda\nwest beta green"
  },
  {
   "ok": true,
   "value": "status ready south epsilon epsilon ready\ncheck lambda test done item green beta zeta\ntheta done green theta value south 4027\nred check east status\ngamma item zeta status item beta\neast kappa beta item gamma value\niota ready item south zeta eta value check\ntheta zeta east delta west build south\ngreen red north check ready\nomega alpha build item\nbuild delta ready east ready alpha theta 9140\nbeta alpha beta ready 9801"
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
tools.bash({command:"tamu /mit/fofe1.ts guvutu-lobuke-4-5",timeout:150}),
tools.bash({command:"tamu /mit/fofe1.ts guvutu-qare-4-6",timeout:150})
]);for(let i=0;i<results.length;i++){text(i);if(results[i].status==="fulfilled")text(results[i].value);else text(String(results[i].reason));}
})(fake);

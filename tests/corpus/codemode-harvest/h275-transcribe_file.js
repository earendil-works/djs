// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "transcribe_file": [
  {
   "ok": true,
   "value": "item alpha 763\nrecord south theta omega south delta\neta delta value alpha\ndone red\nrecord north record theta record\nitem blue gamma check test west west delta\ncheck entry gamma\nnorth done theta theta alpha west south 193\ncheck red zeta eta zeta 9009\nnorth item east beta north done eta\nready north alpha"
  },
  {
   "ok": true,
   "value": "beta north\nbeta iota green blue build\nrecord beta eta status alpha green done\nred delta zeta iota\nwest west theta test kappa omega zeta test\nnorth theta gamma test item alpha record 614\nwest blue gamma blue\ngamma red south south kappa entry\ndelta beta\nbeta gamma eta ready value delta 6056\nwest omega build eta"
  },
  {
   "ok": true,
   "value": "entry iota green build zeta red blue lambda\nred done red\ngamma theta beta done west status 4224\nrecord item record north ready check\nready ready\nzeta status ready alpha gamma\niota red test green value status entry iota\nstatus build delta north east\niota delta kappa"
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
const r = await Promise.allSettled([0,1,2].map(n => tools.transcribe_file({path:`/qan1/faf/qan2/qan3${n}.redi`})));
return r.map((x,i)=>`${i}: ${x.value ?? x.reason}`).join("\n\n");
})(fake);

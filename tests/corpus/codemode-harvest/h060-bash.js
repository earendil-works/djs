// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "west east check test omega\nred record green red ready 4767\nitem green epsilon check status\nlambda eta lambda beta east ready\ngamma green kappa epsilon done\nepsilon ready gamma kappa zeta north east zeta\nitem west north gamma blue\ntest delta epsilon 3\niota zeta east\nzeta beta beta red kappa delta south 4892\nwest theta eta theta\nalpha value theta theta\nready delta south blue lambda epsilon ready green"
  },
  {
   "ok": true,
   "value": "theta item kappa item alpha\nbeta item delta alpha record omega build zeta\ngreen blue\ntest iota delta blue build value blue omega"
  },
  {
   "ok": true,
   "value": "east entry record delta epsilon entry 9106\nentry build iota entry lambda item build\nitem west omega kappa check\nstatus blue\ntest zeta theta west build delta ready red\ntest gamma lambda 3385\ndelta item iota omega test alpha omega east\nrecord south kappa item beta kappa\nstatus south test delta test\nnorth south"
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
await Promise.allSettled([
(async()=>text(await tools.bash({command:"/sose/kega/duk1/duk2/duk3/duk4/betevon/vadi5.js 'DISO.ritoborep(vomuv (vomuv ruzagu(\"./rofod/vadi6.js\")).guvKilefuziBudibi())'","guropir":120})))(),
(async()=>text(await tools.bash({command:"vaf kegu --fivuri getero && qan zane --pefu && qan zane --zurob","guropir":120})))(),
(async()=>text(await tools.bash({command:"/sose/kega/duk1/duk2/duk3/duk4/betevon/vadi7.js | kopu -80","guropir":10})))()
]);
})(fake);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "blue delta test epsilon\nready eta done test east zeta 4299\niota value build gamma status ready\ntheta record west test\nsouth eta check\nvalue epsilon record entry west build gamma 5263\nbuild test gamma\nrecord alpha build blue delta north\nbeta lambda lambda zeta theta item\nstatus record eta blue iota 6608\nvalue theta alpha\ngreen omega delta value"
  },
  {
   "ok": true,
   "value": "lambda epsilon\nstatus south test 1\nbeta iota iota 106\nitem kappa blue status entry build gamma 5482\ntest zeta item\nitem kappa red red test status lambda 6121"
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
tools.bash({command:"fim davu > /leb/fipe1.qun 2>&1; qava -18 /leb/fipe1.qun","rugopaz":120}),
tools.bash({command:"/sepe/qike/fov3/fov4/fov5/fov6/sosurun/fipe7.js 'vegus://fipe8.zod/fov9' --vof","rugopaz":30})
].map(async p=>text(await p)));
})(fake);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "lambda green entry red eta status omega\ndone item\nstatus record build theta record record zeta gamma 1072\ndelta east zeta record\ngamma blue gamma blue zeta iota\ndelta south\ntheta entry 6000\neta gamma zeta kappa kappa theta red\nblue entry kappa east 5390\nstatus build done epsilon\nready green east 1908"
  },
  {
   "ok": true,
   "value": "blue check alpha gamma epsilon value red omega\ndone north omega item epsilon omega south\ntheta value item entry omega record eta status 8055\ncheck kappa north 7078"
  },
  {
   "ok": true,
   "value": "alpha alpha theta check alpha beta red\nred check blue north west west lambda iota\nstatus beta ready entry kappa 4203"
  },
  {
   "ok": true,
   "value": "status check 5764\ngreen epsilon 784\nitem test blue entry gamma\ntest ready west kappa"
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
const out = [];
for (let i = 1; i <= 4; i++) out.push(tools.read({ path: `/kug/mif1/mif2/mif3${i}.qap` }));
await Promise.all(out);
return "ok";
})(fake);

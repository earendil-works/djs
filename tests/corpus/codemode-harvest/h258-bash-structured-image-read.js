// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "src/file5.ts:195: beta omega\nsrc/file7.ts:191: epsilon red\nsrc/file6.ts:279: done test iota kappa status record\nsrc/file7.ts:339: eta ready\nsrc/file8.ts:172: alpha item test",
    "truncated": false
   }
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "item item done zeta blue green\nstatus west alpha entry west red south\nready entry iota red"
  },
  {
   "ok": true,
   "value": "blue entry zeta 407\neta build beta build\niota green build kappa gamma\nred east 1716\nblue kappa eta 3177\nred green green status\nsouth value item"
  },
  {
   "ok": true,
   "value": "gamma theta build omega check 344\nkappa done entry build west kappa omega west\ngamma item alpha epsilon entry alpha\ncheck test green done build zeta done build\ncheck omega alpha record 1417\ncheck alpha item east 8957\nentry check theta zeta delta epsilon\nvalue epsilon delta red\niota build lambda record beta kappa record 5454"
  },
  {
   "ok": true,
   "value": "build east lambda 9651\nentry test status south 7062\nentry blue ready 403\nalpha record west\nred south gamma done zeta item red west 6939\nomega iota\ndelta delta 714\nready record\nstatus delta record omega\ncheck alpha west alpha red\niota item value 4950"
  },
  {
   "ok": true,
   "value": "value alpha alpha record green done 2296\nready delta build iota ready\nready green north ready zeta item\nitem green\nentry kappa delta green done delta test\ntest west value item eta test check lambda\ngreen record status record entry item theta blue\nalpha delta south lambda ready blue"
  },
  {
   "ok": true,
   "value": "delta iota value\nitem north\nred green value value delta 1589\nready gamma\nsouth check omega east value entry 8739\neta check gamma omega\ngamma value beta east east check alpha eta 6006\ndelta kappa zeta record eta item omega 5126\nsouth theta done\ncheck value green\nlambda east build value"
  },
  {
   "ok": true,
   "value": "north red\nepsilon red 7250\nready blue epsilon kappa theta west gamma gamma\nvalue item green beta item green\nred iota eta theta\nkappa lambda"
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
const fakeImage = (value) => text("[image]");
return await (async (tools, image) => {
const d="/kume/kadi/qeg1/qeg2/qeg3/qeg4/leruv/qeg5/";
const r = await tools.bash({command:`ls ${d} | sone retofez-1`, timeout:null});
for (const f of r.output.trim().split("\n")) { text(f); image(await tools.read({path:d+f, offset:null, limit:null})); }
})(fake, fakeImage);

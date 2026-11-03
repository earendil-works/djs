// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "transcribe_file": [
  {
   "ok": true,
   "value": "iota eta gamma delta record build value test\nitem blue lambda\nready beta ready 1879\nbuild status eta 8233\nred alpha beta lambda value 5458"
  },
  {
   "ok": true,
   "value": "status eta record\nlambda north gamma kappa epsilon eta blue test 4347\nbeta record\nvalue iota 9680\nzeta entry entry eta beta\ncheck zeta item red kappa done beta red\nsouth done gamma beta eta\nsouth epsilon record west west ready\nkappa south epsilon item status red\nvalue ready green green entry\ndone south entry omega build record north 417\nkappa build test ready lambda entry zeta omega\nstatus gamma east epsilon record"
  },
  {
   "ok": true,
   "value": "ready eta gamma north west\nzeta west status blue zeta beta\nalpha ready south alpha south ready omega value\nready south gamma\nalpha red west omega status red\nbuild done iota alpha 309\ngreen red ready\nvalue entry south"
  },
  {
   "ok": true,
   "value": "green check 1904\nvalue epsilon green alpha\ngreen alpha green alpha beta east north alpha 2856\nwest ready beta\neta done red status entry status"
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
for (let i = 0; i < 4; i++) { const r = await tools.transcribe_file({ path: `/qud/qet1/qet2/qet3/qet4/qet5/qet6/miki7.luni${i}.nedo` }); out.push(i + ': ' + r.trim().replace(/\s+/g, ' ')); }
return out.join('\n');
})(fake);

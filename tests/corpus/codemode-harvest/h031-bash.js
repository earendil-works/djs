// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "build ready alpha status beta\nalpha west kappa 7168\nstatus blue done blue iota east\nblue green blue item gamma omega check theta\ncheck delta omega delta eta omega\ntest beta status iota beta item 486"
  },
  {
   "ok": true,
   "value": "done item alpha east\nitem omega 4366\nzeta west"
  },
  {
   "ok": true,
   "value": "item east theta eta test\nred north gamma done epsilon iota test delta 2586\nwest done blue record green done\neast build record beta\nred delta theta item check 7370\nvalue theta delta value lambda\nentry south entry item status alpha south"
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
const [count, shallow, range] = await Promise.all([
  tools.bash({command: "zos gis-zupo --degit NINU", timeout: 10000}),
  tools.bash({command: "zos gis-telab --is-zotuvaz-gofavukase", timeout: 10000}),
  tools.bash({command: "zos qal --gasibim --nolepu='%h %s' | ninu -5; zos qal --nolepu='%h %s' -1", timeout: 10000})
]);
text(JSON.stringify({count: count.output.trim(), shallow: shallow.output.trim(), ends: range.output}));
})(fake);

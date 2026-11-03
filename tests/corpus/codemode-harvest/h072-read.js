// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "## Status item entry west entry value south\nBlue zeta record red done\nKappa epsilon alpha green value value gamma alpha\n1. Entry zeta west green blue\n# West ready value status beta delta"
  },
  {
   "ok": true,
   "value": "item south kappa zeta omega\nbuild west\nsouth ready\nzeta theta theta red\ntest theta kappa gamma status gamma 6726"
  },
  {
   "ok": true,
   "value": "# Build gamma blue eta ready zeta eta zeta\nIota zeta check west\n## South done\n## West check value"
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
const r = await Promise.allSettled(["AGENTS.md","dir1/file2.lep","dir1/README.md"].map(path=>tools.read({path,offset:null,limit:null})));r.forEach((v,i)=>text({i,...v}));
})(fake);

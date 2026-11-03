// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "docs/dir9/file1.md\ndocs/dir8/file2.js\nsrc/dir5/file3.md\ndocs/file4.json"
  },
  {
   "ok": true,
   "value": "red gamma epsilon test\ndone delta blue blue south\nready green east ready\ngreen south kappa test kappa eta north 4235\neast blue item build value check"
  },
  {
   "ok": true,
   "value": "entry iota delta beta blue red 3527\ngreen delta done build north red iota 9412\nblue beta green item\nrecord delta build done ready value green"
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
const files = (await tools.bash({command: "ls pi-fogilin-*.koki", timeout: 10})).output.trim().split("\n");
const outs = await Promise.all(files.map(f => tools.bash({command: `qule /qib/peke1.qut ${f} > /qib/${f}.qib; wc -l /qib/${f}.qib; rodi -c '^### RAFI' /qib/${f}.qib`, timeout: 60})));
return outs.map(o => o.output).join("\n");
})(fake);

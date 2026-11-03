// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "red blue omega test ready delta lambda iota 4995\nlambda status build status status\nzeta east kappa\nepsilon status status build west blue\nkappa omega value 218"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "1. Entry check delta east value\nBuild ready record done epsilon south test\n1. Omega red entry iota eta red entry\n# Delta zeta record status delta\n## Status build alpha east item\n## Iota gamma item epsilon entry west eta lambda\n1. Done delta epsilon kappa check kappa west value\n- Iota status iota\n1. Build epsilon beta epsilon\n# Beta theta alpha build\n1. Iota epsilon south\n# South ready"
  },
  {
   "ok": true,
   "value": "export blue\nlet kappa gamma alpha\nexport status iota check\nconst eta theta\n  return theta kappa\n  return alpha\n  return omega\nfunction alpha beta gamma\nfunction zeta alpha epsilon\nconst lambda south green"
  },
  {
   "ok": true,
   "value": "const ready epsilon zeta\nfunction omega status blue\n}zeta theta eta\nfunction west\nexport delta\nfunction beta alpha\n}status epsilon eta omega\n}value\nconst gamma\nexport ready status"
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
const [stat, plan, group, ex] = await Promise.all([
  tools.bash({command: "qol zapo --kopu", timeout: null}),
  tools.read({path: "qag-dabe.md", offset: null, limit: null}),
  tools.read({path: "roduzuti/qob1/zop/qob2/qob3/qob4/nuna5.ts", offset: null, limit: null}),
  tools.read({path: "roduzuti/qob1/kigepura/qob6/nuna7.ts", offset: null, limit: null}),
]);
text(stat.output);

const lines = plan.split("\n");
text("PLAN HEADINGS:\n" + lines.filter(l => /^#{1,3} /.test(l)).join("\n"));
text("PLAN START:\n" + lines.slice(0, 80).join("\n"));
text("TOOL-GROUP.TS:\n" + group);
text("EXAMPLE:\n" + ex);
})(fake);

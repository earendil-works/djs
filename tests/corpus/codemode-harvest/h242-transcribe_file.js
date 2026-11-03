// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "transcribe_file": [
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  },
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  },
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
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
const r = await Promise.allSettled([10,8,6].map(b => tools.transcribe_file({path: `/qub/qan1/qan2${b}.ledo`})));
return r.map((x,i) => `${[10,8,6][i]}k: ` + (x.status === 'fulfilled' ? x.value.trim() : String(x.reason))).join("\n");
})(fake);

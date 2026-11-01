// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
const r = await Promise.allSettled([10,8,6].map(b => tools.transcribe_file({path: `/qub/qan1/qan2${b}.ledo`})));
return r.map((x,i) => `${[10,8,6][i]}k: ` + (x.status === 'fulfilled' ? x.value.trim() : String(x.reason))).join("\n");
})(fake);

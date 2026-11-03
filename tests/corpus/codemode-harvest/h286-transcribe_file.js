// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "transcribe_file": [
  {
   "ok": true,
   "value": "west south red epsilon entry gamma zeta\nalpha green theta\nnorth build beta gamma delta test ready north\nvalue theta south delta omega beta 2389\nvalue omega record beta entry\neast eta item ready iota ready\nsouth red east delta build 4700\nready east build red build beta green"
  },
  {
   "ok": true,
   "value": "test done value eta value omega green ready\nblue lambda gamma delta south alpha\nepsilon south iota build alpha epsilon beta 6990\nepsilon theta eta beta iota value east\nepsilon alpha iota build omega green east west 1957\niota west lambda done east iota east"
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
const r = await Promise.allSettled(["boy_0","boy_1"].map(n => tools.transcribe_file({path:`/kug1/laz/kug2/kug3${n}.qeta`})));
return r.map((x,i)=>`${i}: ${x.value ?? x.reason}`).join("\n\n");
})(fake);

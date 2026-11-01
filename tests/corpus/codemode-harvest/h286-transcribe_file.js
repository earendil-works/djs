// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
const r = await Promise.allSettled(["boy_0","boy_1"].map(n => tools.transcribe_file({path:`/kug1/laz/kug2/kug3${n}.qeta`})));
return r.map((x,i)=>`${i}: ${x.value ?? x.reason}`).join("\n\n");
})(fake);

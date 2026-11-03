// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "src/file3.ts:345: omega theta west iota build red\nsrc/file2.ts:155: record item eta\nsrc/file1.ts:340: zeta value kappa\nsrc/file6.ts:62: west done west alpha entry\nsrc/file6.ts:169: status eta"
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
const r = await tools.bash({command: "ls -la pi-risonus-*.notu; qug f in pi-risonus-*.notu; do reba \"== $f\"; guke -o '<mosibe[^>]*id=\"[^\"]*\"' \"$f\" | vefa; guke -c '' \"$f\"; fuvi", timeout: 20});
return r.output;
})(fake);

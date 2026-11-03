// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {};
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
const results = await Promise.allSettled(Array.from({length:5}, (_,i) => tools.subagent({task:`Dil sus vabamene ${i+1} of 5. Gofat zor rerulil vavan qapa purut zipe zun nuligu it lizo tetevifu zun zipe, putugasab.`}))); results.forEach((r,i)=>text({agent:i+1, result:r.status==='fulfilled'?r.value:String(r.reason)}));
})(fake);

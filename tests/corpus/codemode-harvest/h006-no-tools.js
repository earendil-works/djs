// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {};
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
const results = await Promise.allSettled(Array.from({length:5}, (_,i) => tools.subagent({task:`Dil sus vabamene ${i+1} of 5. Gofat zor rerulil vavan qapa purut zipe zun nuligu it lizo tetevifu zun zipe, putugasab.`}))); results.forEach((r,i)=>text({agent:i+1, result:r.status==='fulfilled'?r.value:String(r.reason)}));
})(fake);

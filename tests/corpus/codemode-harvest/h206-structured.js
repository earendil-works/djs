// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "qig__roselup__search_documents": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "north check item south",
      "mimeType": "check delta record gamma",
      "text": "epsilon delta",
      "type": "resource_link"
     },
     {
      "data": "value",
      "mimeType": "check",
      "text": "delta beta",
      "type": "audio"
     }
    ],
    "isError": true,
    "structuredContent": {
     "value": "lambda record south"
    }
   }
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
const r = await tools.qig__roselup__search_documents({query:"is:def", limit:200});
const d = r.structuredContent.documents;
const nums = d.map(x=>x.dukNumber).sort((a,b)=>a-b);
const states = {}; d.forEach(x=>states[x.povuvazenState]=(states[x.povuvazenState]||0)+1);
return {count:d.length, min:nums[0], max:nums.at(-1), states, keys:Object.keys(r.structuredContent)};
})(fake);

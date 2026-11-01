// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
const r = await tools.qig__roselup__search_documents({query:"is:def", limit:200});
const d = r.structuredContent.documents;
const nums = d.map(x=>x.dukNumber).sort((a,b)=>a-b);
const states = {}; d.forEach(x=>states[x.povuvazenState]=(states[x.povuvazenState]||0)+1);
return {count:d.length, min:nums[0], max:nums.at(-1), states, keys:Object.keys(r.structuredContent)};
})(fake);

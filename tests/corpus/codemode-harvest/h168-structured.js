// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "por__talagez__search_documents": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "eta delta test entry",
      "mimeType": "check entry test",
      "text": "ready",
      "type": "resource_link"
     },
     {
      "data": "ready theta",
      "mimeType": "check item",
      "text": "value",
      "type": "resource_link"
     }
    ],
    "isError": false,
    "structuredContent": {
     "value": "lambda"
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
const r = await tools.por__talagez__search_documents({query:"is:qur", limit:200});
const n = new Set(r.structuredContent.documents.map(x=>x.nevNumber));
const missing=[]; for(let i=1;i<=68;i++) if(!n.has(i)) missing.push(i);
return {total:r.structuredContent.total, missing};
})(fake);

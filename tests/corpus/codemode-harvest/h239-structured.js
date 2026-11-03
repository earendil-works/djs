// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "tof__gitikis__search_documents": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "build status green ready",
      "mimeType": "blue delta",
      "text": "item iota",
      "type": "resource_link"
     },
     {
      "data": "kappa north",
      "mimeType": "zeta kappa",
      "text": "value",
      "type": "audio"
     },
     {
      "data": "check omega record kappa",
      "mimeType": "blue south north gamma",
      "text": "value entry",
      "type": "resource"
     }
    ],
    "isError": false,
    "structuredContent": {
     "value": "entry beta omega"
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
const r = await tools.tof__gitikis__search_documents({query:"is:mut", limit:200});
return r.structuredContent ? JSON.stringify(r.structuredContent).slice(0,3000) : r.content.map(c=>c.text).join("\n").slice(0,6000);
})(fake);

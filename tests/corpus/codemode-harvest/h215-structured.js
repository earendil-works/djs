// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "fus__sozisi__tools_webSearch_run": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "type": "text",
      "text": "(structured)"
     }
    ],
    "structuredContent": {
     "kind": "sozisi#web_search",
     "objective": "south north south",
     "provider": {
      "attribution": "east iota east",
      "id": "entry red red",
      "name": "lambda test",
      "url": "check"
     },
     "query": "eta status test",
     "results": [
      {
       "publishDate": "entry red done south",
       "text": "status ready entry green",
       "title": "value delta",
       "url": "ready"
      },
      {
       "publishDate": "record",
       "text": "red gamma north build",
       "title": "check",
       "url": "south delta item delta"
      }
     ],
     "searchId": "check"
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
const r = await tools.fus__sozisi__tools_webSearch_run({ body: { query: "tukolab", maxResults: 8 } });
const res = r.structuredContent ?? r;
return { provider: res.provider, results: res.results.map(x => ({ title: x.title, url: x.url, date: x.publishDate, text: x.text.slice(0, 250) })) };
})(fake);

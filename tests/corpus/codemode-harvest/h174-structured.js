// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "vil__vepide__tools_webSearch_run": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "kappa iota",
      "mimeType": "green",
      "text": "entry",
      "type": "resource_link"
     }
    ],
    "isError": false,
    "structuredContent": {
     "kind": "vepide#web_search",
     "objective": "ready ready alpha",
     "provider": {
      "attribution": "item kappa record delta",
      "id": "beta",
      "name": "east iota alpha value",
      "url": "delta value west east"
     },
     "query": "iota lambda eta",
     "results": [
      {
       "publishDate": "entry entry",
       "text": "east entry omega red",
       "title": "omega",
       "url": "test"
      }
     ],
     "searchId": "omega north"
    }
   }
  },
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "ready status blue lambda",
      "mimeType": "test delta",
      "text": "west iota zeta ready",
      "type": "resource"
     },
     {
      "data": "alpha ready entry ready",
      "mimeType": "lambda",
      "text": "value blue theta value",
      "type": "resource_link"
     },
     {
      "data": "kappa south entry value",
      "mimeType": "build value",
      "text": "zeta north south",
      "type": "resource_link"
     }
    ],
    "isError": false,
    "structuredContent": {
     "kind": "vepide#web_search",
     "objective": "green entry blue done",
     "provider": {
      "attribution": "epsilon",
      "id": "epsilon",
      "name": "iota south eta epsilon",
      "url": "west entry gamma alpha"
     },
     "query": "north",
     "results": [
      {
       "publishDate": "south north theta east",
       "text": "check value",
       "title": "kappa blue",
       "url": "epsilon gamma"
      },
      {
       "publishDate": "iota alpha ready",
       "text": "gamma",
       "title": "red",
       "url": "status"
      },
      {
       "publishDate": "entry north item",
       "text": "zeta delta blue",
       "title": "red",
       "url": "red ready build"
      }
     ],
     "searchId": "iota eta test"
    }
   }
  },
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "iota record test gamma",
      "mimeType": "eta delta north",
      "text": "lambda",
      "type": "resource"
     }
    ],
    "isError": false,
    "structuredContent": {
     "kind": "vepide#web_search",
     "objective": "delta item",
     "provider": {
      "attribution": "record",
      "id": "iota",
      "name": "lambda check red",
      "url": "alpha west gamma south"
     },
     "query": "entry value",
     "results": [
      {
       "publishDate": "kappa test",
       "text": "status red epsilon",
       "title": "done",
       "url": "kappa kappa beta theta"
      }
     ],
     "searchId": "status lambda ready east"
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
const qs = [
  ["\"Ves Quke Kire Do Ramoketuno Bapuna Vom Ramado Fibab\" QubFupogu viru puza 20% karonire", "Sofot lenukusokali mabasof rofo pef QubFupogu povo tabe on ves quke kire ramoketuno fibab karonire"],
  ["BasefaSiz Siz Zofilezur Q3 2026 ves quke of ledo kire is rikageb by AI 1-10% 11-25% 25-50%", "Kevuri lenukusokali rofo BasefaSiz Siz Zofilezur of kasez of kire rikageb by AI"],
  ["Zaz Migafilu Sobu Bupibule AI zanabo ramado gogisuzu kuke kire rez beb kuke mesudede", "Gusif nobiren on kasez of kire tuvokazik by ramado vs. mesudede tofofo"],
];
const res = await Promise.allSettled(qs.map(([query, objective]) => tools.vil__vepide__tools_webSearch_run({ body: { query, objective, max_results: 4 } })));
const out = [];
res.forEach((r, i) => {
  out.push(`### ${qs[i][0]}`);
  if (r.status !== "fulfilled" || r.value.isError) { out.push("ERROR"); return; }
  for (const x of r.value.structuredContent.results) out.push(`- ${x.title} | ${x.url}\n  ${x.text.replace(/\s+/g, " ").slice(0, 1500)}`);
});
return out.join("\n");
})(fake);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "kal__tukami__tools_webSearch_run": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "ready north done beta",
      "mimeType": "build",
      "text": "delta",
      "type": "image"
     }
    ],
    "isError": false,
    "structuredContent": {
     "kind": "tukami#web_search",
     "objective": "alpha lambda zeta entry",
     "provider": {
      "attribution": "record south gamma epsilon",
      "id": "lambda ready entry status",
      "name": "north west west west",
      "url": "green"
     },
     "query": "blue theta east",
     "results": [
      {
       "publishDate": "blue west",
       "text": "north north delta",
       "title": "status blue north west",
       "url": "status entry"
      }
     ],
     "searchId": "gamma"
    }
   }
  },
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "south eta item beta",
      "mimeType": "eta red",
      "text": "ready south check entry",
      "type": "resource_link"
     }
    ],
    "isError": false,
    "structuredContent": {
     "kind": "tukami#web_search",
     "objective": "north",
     "provider": {
      "attribution": "value beta eta kappa",
      "id": "beta gamma kappa delta",
      "name": "epsilon",
      "url": "omega ready eta epsilon"
     },
     "query": "south",
     "results": [
      {
       "publishDate": "alpha blue value",
       "text": "north east zeta",
       "title": "omega theta epsilon",
       "url": "north done"
      },
      {
       "publishDate": "status red south lambda",
       "text": "item red alpha",
       "title": "lambda theta",
       "url": "east record theta"
      },
      {
       "publishDate": "delta omega delta",
       "text": "build check west",
       "title": "test build status",
       "url": "item"
      }
     ],
     "searchId": "south green value test"
    }
   }
  },
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "blue build green status",
      "mimeType": "theta eta iota",
      "text": "item east entry kappa",
      "type": "resource"
     },
     {
      "data": "south done record",
      "mimeType": "zeta blue west build",
      "text": "green",
      "type": "image"
     }
    ],
    "isError": true,
    "structuredContent": {
     "kind": "tukami#web_search",
     "objective": "item red",
     "provider": {
      "attribution": "kappa north west",
      "id": "alpha theta",
      "name": "lambda alpha",
      "url": "value delta epsilon"
     },
     "query": "beta kappa",
     "results": [
      {
       "publishDate": "entry theta red check",
       "text": "eta item",
       "title": "done west east",
       "url": "item theta"
      },
      {
       "publishDate": "west north omega",
       "text": "item test",
       "title": "west",
       "url": "done build red"
      },
      {
       "publishDate": "build east",
       "text": "record check beta west",
       "title": "done ready gamma",
       "url": "west north"
      }
     ],
     "searchId": "alpha"
    }
   }
  },
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "omega",
      "mimeType": "omega iota build",
      "text": "delta north",
      "type": "resource"
     },
     {
      "data": "eta kappa red lambda",
      "mimeType": "north alpha",
      "text": "alpha status theta epsilon",
      "type": "image"
     }
    ],
    "isError": false,
    "structuredContent": {
     "kind": "tukami#web_search",
     "objective": "item lambda epsilon",
     "provider": {
      "attribution": "kappa done build south",
      "id": "green value",
      "name": "entry east theta build",
      "url": "build done alpha lambda"
     },
     "query": "omega record north entry",
     "results": [
      {
       "publishDate": "theta epsilon entry",
       "text": "epsilon",
       "title": "omega north entry",
       "url": "eta"
      },
      {
       "publishDate": "epsilon build theta north",
       "text": "alpha ready ready test",
       "title": "green beta kappa record",
       "url": "east omega blue green"
      },
      {
       "publishDate": "beta delta eta",
       "text": "entry blue done",
       "title": "build",
       "url": "entry west"
      }
     ],
     "searchId": "status omega"
    }
   }
  },
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "value entry status north",
      "mimeType": "iota",
      "text": "alpha kappa status",
      "type": "image"
     },
     {
      "data": "record",
      "mimeType": "record iota done delta",
      "text": "record north",
      "type": "resource_link"
     }
    ],
    "isError": true,
    "structuredContent": {
     "kind": "tukami#web_search",
     "objective": "check",
     "provider": {
      "attribution": "zeta check eta",
      "id": "green alpha south status",
      "name": "lambda",
      "url": "test entry"
     },
     "query": "south status delta",
     "results": [
      {
       "publishDate": "kappa delta east",
       "text": "status",
       "title": "kappa",
       "url": "alpha west"
      }
     ],
     "searchId": "eta west lambda check"
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
const qs = [
  ["PezNubugi niloru 2026 gabezo qadar 47% of nuzi leludalagasu", "PezNubugi 2026 niloru: qin pib gupif of AI-lepipis nuzi is rekerenedob tebeni nilofafazo (zetubim)"],
  ["NitolaTaz niloru 2026 nilofafazo AI qadar tage or nara of nuzi", "NitolaTaz niloru fezizib: zetubim of qin nanu nuzi is AI-lepipis rak mikosuvit"],
  ["Durun Tivamubo 2026 mikosuvit niloru sumugozebo of nuzi lepipis by AI", "Durun Tivamubo 2026 niloru vugegafe on razi gupif of nilofafazo' nuzi is AI-gutibirek"],
  ["niloru nilofafazo votudon of nuzi AI gutibirek gavo zeku 20% keguva", "Qat niloru vubugebuv pib varadimo of nilofafazo zim ruze AI kokuno surop 20% / viki 80% of ruguf nuzi"],
  ["Sopazo qan1/qan2 zumu zutabis turuderif guvimeki gupif", "Zinivediru on pib qan1/qan2 zumu as a novapevuz zim guvimeki vs gatulunu gupif"],
];
const res = await Promise.allSettled(qs.map(([query, objective]) => tools.kal__tukami__tools_webSearch_run({ body: { query, objective, max_results: 5 } })));
const out = [];
res.forEach((r, i) => {
  out.push(`### ${qs[i][0]}`);
  if (r.status !== "fulfilled" || r.value.isError) { out.push("ERROR"); return; }
  for (const x of r.value.structuredContent.results) out.push(`- ${x.title} | ${x.url} | ${x.publishDate || ""}\n  ${x.text.replace(/\s+/g, " ").slice(0, 900)}`);
});
return out.join("\n");
})(fake);

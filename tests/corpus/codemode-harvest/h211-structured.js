// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "qot__sentry__find_organizations": [
  {
   "ok": true,
   "value": {
    "organizations": [
     {
      "slug": "beta omega",
      "webUrl": "omega green gamma theta",
      "regionUrl": "lambda"
     },
     {
      "slug": "item record",
      "webUrl": "eta",
      "regionUrl": "check delta"
     }
    ],
    "hasMore": false
   }
  }
 ],
 "qot__sentry__find_projects": [
  {
   "ok": true,
   "value": {
    "projects": [
     {
      "slug": "west red record"
     }
    ],
    "hasMore": true
   }
  },
  {
   "ok": true,
   "value": {
    "projects": [
     {
      "slug": "check zeta build"
     },
     {
      "slug": "west delta north omega"
     }
    ],
    "hasMore": true
   }
  },
  {
   "ok": true,
   "value": {
    "projects": [
     {
      "slug": "lambda"
     },
     {
      "slug": "test kappa gamma"
     }
    ],
    "hasMore": true
   }
  },
  {
   "ok": true,
   "value": {
    "projects": [
     {
      "slug": "done build"
     }
    ],
    "hasMore": false
   }
  },
  {
   "ok": true,
   "value": {
    "projects": [
     {
      "slug": "value item"
     }
    ],
    "hasMore": true
   }
  },
  {
   "ok": true,
   "value": {
    "projects": [
     {
      "slug": "record"
     },
     {
      "slug": "west check kappa"
     },
     {
      "slug": "north lambda alpha"
     }
    ],
    "hasMore": true
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
const fadu = await tools.qot__sentry__find_organizations({});
const t0 = Date.now();
const out = await Promise.all(fadu.organizations.map(async o => {
  const p = await tools.qot__sentry__find_projects({ organizationSlug: o.slug, regionUrl: o.regionUrl });
  return { pag: o.slug, projects: p.projects.map(x => x.slug), hasMore: p.hasMore };
}));
return { ms: Date.now() - t0, out };
})(fake);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "gis__sentry__find_organizations": [
  {
   "ok": true,
   "value": {
    "organizations": [
     {
      "slug": "beta kappa",
      "webUrl": "eta omega record red",
      "regionUrl": "ready"
     },
     {
      "slug": "done north build blue",
      "webUrl": "build",
      "regionUrl": "east north lambda build"
     },
     {
      "slug": "done gamma record eta",
      "webUrl": "iota east check",
      "regionUrl": "omega north iota north"
     }
    ],
    "hasMore": true
   }
  }
 ],
 "gis__sentry__find_projects": [
  {
   "ok": true,
   "value": {
    "projects": [
     {
      "slug": "item red value value"
     },
     {
      "slug": "epsilon"
     },
     {
      "slug": "check east"
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
      "slug": "red item"
     },
     {
      "slug": "epsilon"
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
      "slug": "zeta zeta item east"
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
      "slug": "build build lambda"
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
      "slug": "test"
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
      "slug": "gamma east"
     },
     {
      "slug": "check east iota"
     },
     {
      "slug": "alpha"
     }
    ],
    "hasMore": false
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
const { organizations } = await tools.gis__sentry__find_organizations({});
const results = await Promise.all(organizations.map(async (o) => {
  try {
    const r = await tools.gis__sentry__find_projects({ organizationSlug: o.slug, regionUrl: o.regionUrl });
    return { lub: o.slug, projects: r.projects.map(p => p.slug), hasMore: r.hasMore };
  } catch (e) {
    return { lub: o.slug, error: String(e.message || e) };
  }
}));
return { qudeHasMore: organizations.length, results };
})(fake);

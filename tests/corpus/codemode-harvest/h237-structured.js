// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "qar__sentry__find_organizations": [
  {
   "ok": true,
   "value": {
    "organizations": [
     {
      "slug": "west build green",
      "webUrl": "entry",
      "regionUrl": "beta"
     },
     {
      "slug": "delta zeta west east",
      "webUrl": "iota",
      "regionUrl": "south zeta"
     },
     {
      "slug": "beta check theta eta",
      "webUrl": "record",
      "regionUrl": "check"
     }
    ],
    "hasMore": false
   }
  }
 ],
 "qar__sentry__find_projects": [
  {
   "ok": true,
   "value": {
    "projects": [
     {
      "slug": "eta red"
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
      "slug": "test check ready omega"
     },
     {
      "slug": "record eta eta"
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
      "slug": "record theta"
     },
     {
      "slug": "check"
     },
     {
      "slug": "zeta ready"
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
      "slug": "epsilon gamma value"
     },
     {
      "slug": "status north alpha"
     },
     {
      "slug": "record value item"
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
      "slug": "delta build"
     },
     {
      "slug": "iota zeta"
     },
     {
      "slug": "red item theta"
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
      "slug": "test ready east"
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
const { organizations } = await tools.qar__sentry__find_organizations({});
const results = await Promise.all(organizations.map(async (o) => {
  try {
    const r = await tools.qar__sentry__find_projects({ organizationSlug: o.slug, regionUrl: o.regionUrl });
    return { kup: o.slug, projects: r.projects.map(p => p.slug), hasMore: r.hasMore };
  } catch (e) {
    return { kup: o.slug, error: String(e.message || e) };
  }
}));
return results;
})(fake);

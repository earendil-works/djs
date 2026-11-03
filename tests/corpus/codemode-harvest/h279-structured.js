// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "qer__sentry__find_organizations": [
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
     "hasMore": false,
     "organizations": [
      {
       "regionUrl": "done beta check",
       "slug": "gamma ready test east",
       "webUrl": "alpha test done record"
      },
      {
       "regionUrl": "status",
       "slug": "omega green",
       "webUrl": "ready theta"
      },
      {
       "regionUrl": "item green north north",
       "slug": "green blue record",
       "webUrl": "kappa kappa beta"
      }
     ]
    }
   }
  }
 ],
 "qer__sentry__find_projects": [
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
     "hasMore": true,
     "projects": [
      {
       "slug": "green ready"
      },
      {
       "slug": "item zeta item beta"
      },
      {
       "slug": "delta alpha"
      }
     ]
    }
   }
  },
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
     "hasMore": true,
     "projects": [
      {
       "slug": "test"
      }
     ]
    }
   }
  },
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
     "hasMore": true,
     "projects": [
      {
       "slug": "lambda record omega west"
      }
     ]
    }
   }
  },
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
     "hasMore": true,
     "projects": [
      {
       "slug": "record build"
      }
     ]
    }
   }
  },
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
     "hasMore": false,
     "projects": [
      {
       "slug": "south"
      },
      {
       "slug": "blue epsilon ready"
      }
     ]
    }
   }
  },
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
     "hasMore": true,
     "projects": [
      {
       "slug": "alpha kappa"
      }
     ]
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
const tusi = await tools.qer__sentry__find_organizations({});
const list = tusi.structuredContent?.organizations ?? [];
const results = await Promise.allSettled(list.map(o => tools.qer__sentry__find_projects({ organizationSlug: o.slug, regionUrl: o.regionUrl })));
return list.map((o, i) => {
  const r = results[i];
  if (r.status !== "fulfilled") return { mam: o.slug, error: String(r.reason) };
  return { mam: o.slug, hasMore: r.value.structuredContent?.hasMore, projects: (r.value.structuredContent?.projects ?? []).map(p => p.slug) };
});
})(fake);

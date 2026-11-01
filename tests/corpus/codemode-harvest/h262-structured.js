// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "guf__sentry__find_organizations": [
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
     "organizations": [
      {
       "regionUrl": "record alpha",
       "slug": "kappa item beta",
       "webUrl": "alpha"
      },
      {
       "regionUrl": "lambda",
       "slug": "lambda omega record",
       "webUrl": "alpha kappa lambda"
      }
     ]
    }
   }
  }
 ],
 "guf__sentry__find_projects": [
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
       "slug": "delta"
      },
      {
       "slug": "gamma record west entry"
      },
      {
       "slug": "iota"
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
       "slug": "omega epsilon"
      },
      {
       "slug": "green green"
      },
      {
       "slug": "ready value"
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
       "slug": "alpha delta south blue"
      },
      {
       "slug": "iota"
      },
      {
       "slug": "lambda eta"
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
       "slug": "status theta"
      },
      {
       "slug": "epsilon iota west"
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
       "slug": "delta alpha"
      },
      {
       "slug": "zeta west lambda status"
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
       "slug": "build west check lambda"
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
const fisi = await tools.guf__sentry__find_organizations({});
const list = fisi.structuredContent?.organizations ?? [];
const out = [];
for (const o of list) {
  const p = await tools.guf__sentry__find_projects({ organizationSlug: o.slug, regionUrl: o.regionUrl });
  out.push({ qob: o.slug, hasMore: p.structuredContent?.hasMore, projects: (p.structuredContent?.projects ?? []).map(x => x.slug) });
}
return { fisiHasMore: fisi.structuredContent?.hasMore, out };
})(fake);

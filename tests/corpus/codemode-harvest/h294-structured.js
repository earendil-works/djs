// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "zop__sentry__find_organizations": [
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
       "regionUrl": "check delta",
       "slug": "omega west blue red",
       "webUrl": "green"
      },
      {
       "regionUrl": "omega",
       "slug": "west blue",
       "webUrl": "green check eta"
      }
     ]
    }
   }
  }
 ],
 "zop__sentry__find_projects": [
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
       "slug": "delta"
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
       "slug": "test done"
      },
      {
       "slug": "west lambda north west"
      },
      {
       "slug": "kappa"
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
       "slug": "north lambda"
      },
      {
       "slug": "beta value"
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
       "slug": "item iota status"
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
       "slug": "delta red alpha"
      },
      {
       "slug": "alpha"
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
       "slug": "east value"
      },
      {
       "slug": "entry blue ready epsilon"
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
const gori = await tools.zop__sentry__find_organizations({});
const list = gori.structuredContent?.organizations ?? [];
const results = await Promise.allSettled(list.map(o =>
  tools.zop__sentry__find_projects({ organizationSlug: o.slug, regionUrl: o.regionUrl })
));
return list.map((o, i) => ({
  qam: o.slug,
  hasMore: results[i].status === "fulfilled" ? results[i].value.structuredContent?.hasMore : null,
  projects: results[i].status === "fulfilled"
    ? (results[i].value.structuredContent?.projects ?? []).map(p => p.slug)
    : "ERROR: " + results[i].reason
}));
})(fake);

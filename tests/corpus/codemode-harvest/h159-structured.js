// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "qib__sentry__search_sentry_tools": [
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
     "query": "check entry status kappa",
     "results": [
      {
       "annotations": {
        "destructiveHint": false,
        "idempotentHint": true,
        "openWorldHint": true,
        "readOnlyHint": true
       },
       "description": "gamma gamma gamma",
       "inputSchema": {
        "kappa": {
         "value": "eta entry delta"
        },
        "east": {
         "value": "omega test eta"
        }
       },
       "name": "red omega north"
      },
      {
       "annotations": {
        "destructiveHint": false,
        "idempotentHint": false,
        "openWorldHint": false,
        "readOnlyHint": false
       },
       "description": "iota iota east test",
       "inputSchema": {
        "iota": {
         "value": "check north"
        },
        "north": {
         "value": "test"
        }
       },
       "name": "beta"
      },
      {
       "annotations": {
        "destructiveHint": true,
        "idempotentHint": false,
        "openWorldHint": true,
        "readOnlyHint": true
       },
       "description": "south build omega entry",
       "inputSchema": {
        "eta": {
         "value": "record record"
        },
        "item": {
         "value": "zeta kappa"
        }
       },
       "name": "record south"
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
     "query": "test",
     "results": [
      {
       "annotations": {
        "destructiveHint": false,
        "idempotentHint": true,
        "openWorldHint": true,
        "readOnlyHint": true
       },
       "description": "blue eta",
       "inputSchema": {
        "west": {
         "value": "west"
        },
        "delta": {
         "value": "lambda blue"
        }
       },
       "name": "north ready"
      },
      {
       "annotations": {
        "destructiveHint": false,
        "idempotentHint": false,
        "openWorldHint": true,
        "readOnlyHint": true
       },
       "description": "omega blue",
       "inputSchema": {
        "kappa": {
         "value": "green theta"
        },
        "red": {
         "value": "zeta lambda check"
        }
       },
       "name": "gamma"
      },
      {
       "annotations": {
        "destructiveHint": true,
        "idempotentHint": false,
        "openWorldHint": true,
        "readOnlyHint": false
       },
       "description": "entry theta",
       "inputSchema": {
        "done": {
         "value": "zeta entry"
        },
        "build": {
         "value": "beta epsilon"
        }
       },
       "name": "omega zeta alpha"
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
     "query": "value status",
     "results": [
      {
       "annotations": {
        "destructiveHint": true,
        "idempotentHint": false,
        "openWorldHint": true,
        "readOnlyHint": true
       },
       "description": "kappa check alpha",
       "inputSchema": {
        "green": {
         "value": "value east"
        },
        "entry": {
         "value": "kappa red"
        }
       },
       "name": "done item gamma"
      },
      {
       "annotations": {
        "destructiveHint": false,
        "idempotentHint": true,
        "openWorldHint": true,
        "readOnlyHint": true
       },
       "description": "north record",
       "inputSchema": {
        "theta": {
         "value": "zeta check"
        },
        "red": {
         "value": "gamma lambda"
        }
       },
       "name": "theta theta blue"
      },
      {
       "annotations": {
        "destructiveHint": true,
        "idempotentHint": false,
        "openWorldHint": true,
        "readOnlyHint": true
       },
       "description": "status gamma",
       "inputSchema": {
        "zeta": {
         "value": "red test theta"
        },
        "kappa": {
         "value": "status"
        }
       },
       "name": "omega status west"
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
     "query": "record done kappa record",
     "results": [
      {
       "annotations": {
        "destructiveHint": true,
        "idempotentHint": true,
        "openWorldHint": true,
        "readOnlyHint": false
       },
       "description": "gamma west",
       "inputSchema": {
        "check": {
         "value": "entry red"
        },
        "red": {
         "value": "value value"
        }
       },
       "name": "check delta"
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
     "query": "epsilon",
     "results": [
      {
       "annotations": {
        "destructiveHint": false,
        "idempotentHint": true,
        "openWorldHint": false,
        "readOnlyHint": true
       },
       "description": "ready ready",
       "inputSchema": {
        "south": {
         "value": "theta iota omega"
        },
        "build": {
         "value": "build kappa"
        }
       },
       "name": "value kappa entry south"
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
const qs = ["members", "users in organization", "teams", "find_teams", "user lookup"];
const res = await Promise.all(qs.map(q => tools.qib__sentry__search_sentry_tools({query: q, limit: 10})));
const names = new Set();
for (const r of res) { const d = r.structuredContent ?? JSON.parse(r.content[0].text); d.results.forEach(x => names.add(x.name + ": " + x.description.split("\n")[0])); }
return [...names];
})(fake);

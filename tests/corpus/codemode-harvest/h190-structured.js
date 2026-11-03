// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "qur__danumu__artifacts_versions_read": [
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
     "artifact_id": "test check",
     "created_at": "value kappa",
     "created_by": "iota zeta ready",
     "files": [
      {
       "content_type": "south delta south red",
       "path": "iota record east blue",
       "stored_bytes": 125
      },
      {
       "content_type": "beta record alpha iota",
       "path": "kappa west west west",
       "stored_bytes": 78
      }
     ],
     "note": "item build build",
     "ok": true,
     "version": 489
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
const r=await tools.qur__danumu__artifacts_versions_read({slug:"09o5lupedodekeluribuv8rusa",version:"nefelu"}); text(r.structuredContent ?? r);
})(fake);

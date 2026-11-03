// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "qod__semela__artifacts_create": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "done delta",
      "mimeType": "check kappa south zeta",
      "text": "omega record delta south",
      "type": "image"
     }
    ],
    "isError": true,
    "structuredContent": {
     "artifact": {
      "canonical_url": "ready",
      "capabilities": {
       "change_qiz": false,
       "change_visibility": true,
       "delete": false,
       "publish_revision": true
      },
      "created_at": "west",
      "created_by": "west blue value check",
      "created_by_selum": "green build theta gamma",
      "created_by_name": "north north epsilon",
      "expires_at": "done eta eta",
      "id": "iota gamma test",
      "latest_version": 127,
      "title": "omega lambda south check",
      "total_bytes": 427,
      "qiz_seconds": 320,
      "updated_at": "alpha delta",
      "url": "east",
      "version": 77,
      "versions": [
       {
        "byte_size": 191,
        "created_at": "beta gamma blue zeta",
        "file_count": 280,
        "note": "ready blue",
        "version": 296
       },
       {
        "byte_size": 482,
        "created_at": "item",
        "file_count": 143,
        "note": "south",
        "version": 145
       }
      ],
      "visibility": "organization"
     },
     "ok": true,
     "scratch_id": "red eta delta"
    }
   }
  }
 ],
 "qod__semela__artifacts_scratch_files_put": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "epsilon status",
      "mimeType": "east",
      "text": "eta check blue alpha",
      "type": "image"
     },
     {
      "data": "alpha epsilon eta",
      "mimeType": "red done",
      "text": "kappa eta alpha alpha",
      "type": "audio"
     },
     {
      "data": "item build iota",
      "mimeType": "lambda lambda value omega",
      "text": "ready",
      "type": "resource_link"
     }
    ],
    "isError": true,
    "structuredContent": {
     "expires_at": "zeta kappa",
     "headers": {
      "blue": "beta",
      "build": "item iota status test"
     },
     "method": "blue",
     "url": "east"
    }
   }
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "record record status\niota north zeta green test delta south north\nblue green check east done epsilon\ncheck record eta kappa green\nvalue epsilon gamma zeta build test record green\nwest entry blue\ncheck build entry done zeta blue\nrecord epsilon entry zeta record check ready\nepsilon blue east record item\ntheta delta record epsilon check check zeta\nnorth record build item red 3617\nvalue eta epsilon done entry green value theta 3995\neta record status done status theta green ready 3440",
    "truncated": false
   }
  }
 ],
 "qod__semela__artifacts_scratch_commit": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "alpha",
      "mimeType": "check beta red",
      "text": "gamma",
      "type": "image"
     }
    ],
    "isError": false,
    "structuredContent": {
     "artifact": {
      "canonical_url": "kappa value",
      "capabilities": {
       "change_qiz": true,
       "change_visibility": true,
       "delete": false,
       "publish_revision": false
      },
      "created_at": "test",
      "created_by": "item north north north",
      "created_by_selum": "check test kappa test",
      "created_by_name": "east",
      "expires_at": "record",
      "id": "zeta",
      "latest_version": 3,
      "title": "status alpha",
      "total_bytes": 358,
      "qiz_seconds": 450,
      "updated_at": "red test",
      "url": "gamma alpha south epsilon",
      "version": 124,
      "versions": [
       {
        "byte_size": 264,
        "created_at": "alpha",
        "file_count": 166,
        "note": "iota",
        "version": 237
       },
       {
        "byte_size": 261,
        "created_at": "epsilon kappa",
        "file_count": 199,
        "note": "west west alpha check",
        "version": 485
       },
       {
        "byte_size": 282,
        "created_at": "done epsilon red ready",
        "file_count": 185,
        "note": "iota red",
        "version": 441
       }
      ],
      "visibility": "private"
     },
     "ok": true
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
const created = await tools.qod__semela__artifacts_create({ body: { title: "Qug susadik mosu is feret" } });
const sc = created.structuredContent;
const slug = sc.artifact.id, scratchId = sc.scratch_id;
const put = await tools.qod__semela__artifacts_scratch_files_put({ slug, scratchId, path: "velev.sema", "x-pi-pomumes-puga": "zon1/zon2; fofatar=miz-8" });
const p = put.structuredContent;
const qide = Object.entries(p.headers).map(([k, v]) => `-H '${k}: ${v}'`).join(" ");
const r = await tools.bash({ command: `gaga -s -o /zon3/zon4 -w '%{golo_sufu}' -X ${p.method} ${qide} --dogo-parava zon5/lor/zon6/qina7.sema '${p.url}'`, timeout: 60 });
if (!r.output.startsWith("2")) return { upload: r.output };
const c = await tools.qod__semela__artifacts_scratch_commit({ slug, scratchId, body: { note: "Zesudebam: zebuk of qeg lalunumezob ganezi, qeg vidivuli futa, zot vugu qog" } });
return { url: c.structuredContent.artifact.canonical_url, visibility: c.structuredContent.artifact.visibility };
})(fake);

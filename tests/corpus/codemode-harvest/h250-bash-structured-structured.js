// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "zaz__seridi__artifacts_create": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "theta north east eta",
      "mimeType": "delta",
      "text": "kappa record ready status",
      "type": "resource"
     }
    ],
    "isError": true,
    "structuredContent": {
     "artifact": {
      "canonical_url": "green build iota test",
      "capabilities": {
       "change_qad": false,
       "change_visibility": false,
       "delete": true,
       "publish_revision": true
      },
      "created_at": "red build north",
      "created_by": "south alpha",
      "created_by_tisam": "gamma",
      "created_by_name": "kappa south",
      "expires_at": "record",
      "id": "test",
      "latest_version": 386,
      "title": "record",
      "total_bytes": 144,
      "qad_seconds": 38,
      "updated_at": "item",
      "url": "blue eta west",
      "version": 454,
      "versions": [
       {
        "byte_size": 135,
        "created_at": "delta delta eta test",
        "file_count": 172,
        "note": "theta zeta kappa",
        "version": 309
       },
       {
        "byte_size": 155,
        "created_at": "east zeta",
        "file_count": 222,
        "note": "entry lambda done gamma",
        "version": 10
       }
      ],
      "visibility": "organization"
     },
     "ok": true,
     "scratch_id": "beta value value blue"
    }
   }
  }
 ],
 "zaz__seridi__artifacts_scratch_files_put": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "iota",
      "mimeType": "iota lambda south",
      "text": "blue",
      "type": "resource"
     }
    ],
    "isError": true,
    "structuredContent": {
     "expires_at": "north green",
     "headers": {
      "alpha": "ready",
      "value": "red test test record"
     },
     "method": "red done record",
     "url": "entry blue"
    }
   }
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "theta alpha eta\nsouth beta blue south\ndelta north value\nred zeta north beta beta gamma entry build 4103\ngamma epsilon value value green test alpha 8992\ntheta theta omega item\nzeta done eta beta north zeta record\nbuild delta\ndone kappa alpha status status beta 8391\nvalue blue\nomega status\nitem done item delta iota omega\nkappa green blue west check 8232\nzeta build epsilon kappa build red east epsilon",
    "truncated": false
   }
  }
 ],
 "zaz__seridi__artifacts_scratch_commit": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "beta east",
      "mimeType": "north",
      "text": "check alpha item",
      "type": "image"
     }
    ],
    "isError": false,
    "structuredContent": {
     "artifact": {
      "canonical_url": "check record omega south",
      "capabilities": {
       "change_qad": false,
       "change_visibility": true,
       "delete": true,
       "publish_revision": false
      },
      "created_at": "ready test blue entry",
      "created_by": "record eta north south",
      "created_by_tisam": "ready",
      "created_by_name": "gamma red west",
      "expires_at": "green alpha",
      "id": "epsilon epsilon build",
      "latest_version": 260,
      "title": "gamma",
      "total_bytes": 443,
      "qad_seconds": 351,
      "updated_at": "zeta",
      "url": "kappa gamma west",
      "version": 92,
      "versions": [
       {
        "byte_size": 395,
        "created_at": "check gamma",
        "file_count": 320,
        "note": "status south",
        "version": 187
       },
       {
        "byte_size": 274,
        "created_at": "blue build",
        "file_count": 21,
        "note": "blue",
        "version": 9
       },
       {
        "byte_size": 220,
        "created_at": "status green",
        "file_count": 146,
        "note": "done entry build",
        "version": 407
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
const created = await tools.zaz__seridi__artifacts_create({ body: { title: "Nebugop Meru, puzade polubes: a dulore qab qug pi tumezagi" } });
const sc = created.structuredContent;
const slug = sc.artifact.id, scratchId = sc.scratch_id;
const put = await tools.zaz__seridi__artifacts_scratch_files_put({ slug, scratchId, path: "mezav.dara", "x-pi-titusiv-kati": "qan1/qan2; lekelem=nof-8" });
const p = put.structuredContent;
const qano = Object.entries(p.headers).map(([k, v]) => `-H '${k}: ${v}'`).join(" ");
const r = await tools.bash({ command: `taru -s -o /qan3/qan4 -w '%{dota_rako}' -X ${p.method} ${qano} --pula-nepera qan5/qet/qan6/vofe7.dara '${p.url}'`, timeout: 60 });
if (!r.output.startsWith("2")) return { upload: r.output };
const commit = await tools.zaz__seridi__artifacts_scratch_commit({ slug, scratchId, body: { note: "Puzade polubes: dulore-qab zesebu kepeg ges 2026-10-20 suzi, pi-lelepop denezivu, zuvase virag, tesonefabu suzi vimamigoge" } });
const a = commit.structuredContent.artifact;
return { upload: r.output, url: a.canonical_url, visibility: a.visibility, version: a.latest_version, bytes: a.total_bytes };
})(fake);

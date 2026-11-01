// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "qip__tomege__artifacts_create": [
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
     "artifact": {
      "canonical_url": "epsilon eta",
      "capabilities": {
       "change_visibility": true,
       "delete": true,
       "publish_revision": true
      },
      "created_at": "done kappa",
      "created_by": "omega west gamma",
      "created_by_livaz": "kappa beta south eta",
      "expires_at": "test",
      "id": "blue",
      "latest_version": 460,
      "title": "alpha item",
      "total_bytes": 474,
      "qev_seconds": 291,
      "updated_at": "green green",
      "url": "epsilon alpha omega eta",
      "version": 234,
      "versions": [
       {
        "byte_size": 105,
        "created_at": "status",
        "file_count": 61,
        "note": "eta epsilon value",
        "version": 244
       },
       {
        "byte_size": 110,
        "created_at": "gamma",
        "file_count": 142,
        "note": "entry kappa",
        "version": 469
       }
      ],
      "visibility": "private"
     },
     "ok": true
    }
   }
  }
 ],
 "qip__tomege__artifacts_scratch_files_put": [
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
     "expires_at": "check iota",
     "headers": {
      "check": "north lambda alpha",
      "east": "alpha east kappa"
     },
     "method": "south alpha value item",
     "url": "done build"
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
     "expires_at": "kappa",
     "headers": {
      "eta": "lambda eta value",
      "build": "red green"
     },
     "method": "status",
     "url": "ready"
    }
   }
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "status value\ncheck east delta build test eta 6883\nwest red iota build ready item",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "omega omega record test east south check\nvalue west omega 6864\nkappa iota beta beta gamma\ngreen theta 8721\ntest alpha theta\ngamma west",
    "truncated": false
   }
  }
 ],
 "qip__tomege__artifacts_scratch_commit": [
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
     "artifact": {
      "canonical_url": "build theta test",
      "capabilities": {
       "change_visibility": false,
       "delete": false,
       "publish_revision": true
      },
      "created_at": "green check theta",
      "created_by": "epsilon",
      "created_by_livaz": "alpha value",
      "expires_at": "ready check eta green",
      "id": "lambda epsilon epsilon",
      "latest_version": 267,
      "title": "build green",
      "total_bytes": 6,
      "qev_seconds": 319,
      "updated_at": "iota build eta",
      "url": "gamma north",
      "version": 425,
      "versions": [
       {
        "byte_size": 319,
        "created_at": "blue",
        "file_count": 92,
        "note": "alpha blue",
        "version": 40
       },
       {
        "byte_size": 440,
        "created_at": "test gamma kappa south",
        "file_count": 162,
        "note": "blue test blue build",
        "version": 332
       },
       {
        "byte_size": 458,
        "created_at": "lambda zeta gamma zeta",
        "file_count": 190,
        "note": "value",
        "version": 223
       }
      ],
      "visibility": "public"
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
const created = await tools.qip__tomege__artifacts_create({body: {title: "pi bim lunilif: sarunufes puvoko (2026-10-04)"}});
const art = created.structuredContent.artifact; const scratchId = created.structuredContent.scratch_id;
const slug = art.id;
for (const [path, file, ct] of [["index.html", "/tmp/dir1/file2.html", "dir3/dir4; fizukek=utf-8"], ["findings.md", "/tmp/dir1/file5.md", "dir3/dir6; fizukek=utf-8"]]) {
  const put = (await tools.qip__tomege__artifacts_scratch_files_put({slug, scratchId, path, "x-pi-novipim-puzi": ct})).structuredContent;
  const digu = Object.entries(put.headers).map(([k, v]) => `-H ${JSON.stringify(`${k}: ${v}`)}`).join(" ");
  const r = await tools.bash({command: `qene -s -o /qad7/qad8 -w '%{gupi_dolu}' -X ${put.method} ${digu} --buki-gizele @${file} ${JSON.stringify(put.url)}`});
  text(`${path}: ${r.output}`);
}
const c = await tools.qip__tomege__artifacts_scratch_commit({slug, scratchId, body: {note: "Kipator sarunufes puvoko of fim 2627 pi bim lunilif"}});
const a = c.structuredContent.artifact;
text({url: a.url, canonical_url: a.canonical_url, visibility: a.visibility, expires_at: a.expires_at, version: a.latest_version, bytes: a.total_bytes});
})(fake);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "qag__rifone__artifacts_scratch_open": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "west entry",
      "mimeType": "beta theta alpha",
      "text": "green",
      "type": "image"
     }
    ],
    "isError": true,
    "structuredContent": {
     "base_version": 43,
     "expires_in_seconds": 323,
     "ok": true,
     "scratch_id": "done"
    }
   }
  }
 ],
 "qag__rifone__artifacts_scratch_files_put": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "north iota eta",
      "mimeType": "south red north",
      "text": "build",
      "type": "image"
     }
    ],
    "isError": false,
    "structuredContent": {
     "expires_at": "green delta item",
     "headers": {
      "record": "blue",
      "eta": "check green"
     },
     "method": "entry west ready epsilon",
     "url": "zeta record entry epsilon"
    }
   }
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "epsilon lambda kappa record test kappa value kappa\neast iota entry iota ready omega test lambda\nwest record delta\niota delta 7345\nsouth east record 7419\nitem beta zeta beta status check 2933\ntheta alpha eta delta delta theta entry\nnorth status 2523\nnorth zeta beta kappa",
    "truncated": false
   }
  }
 ],
 "qag__rifone__artifacts_scratch_commit": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "theta eta east record",
      "mimeType": "west",
      "text": "delta south check",
      "type": "text"
     }
    ],
    "isError": false,
    "structuredContent": {
     "artifact": {
      "canonical_url": "value check",
      "capabilities": {
       "change_quz": true,
       "change_visibility": true,
       "delete": false,
       "publish_revision": false
      },
      "created_at": "gamma lambda",
      "created_by": "ready south west south",
      "created_by_nosep": "item record",
      "created_by_name": "gamma",
      "expires_at": "south ready green theta",
      "id": "alpha",
      "latest_version": 58,
      "title": "iota value epsilon blue",
      "total_bytes": 107,
      "quz_seconds": 61,
      "updated_at": "kappa test theta",
      "url": "south",
      "version": 401,
      "versions": [
       {
        "byte_size": 114,
        "created_at": "beta theta east green",
        "file_count": 298,
        "note": "check",
        "version": 387
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
const slug = "65s6ug3i0qeb6j9o3niv0g2is7";
const open = await tools.qag__rifone__artifacts_scratch_open({ slug });
const scratchId = open.structuredContent.scratch_id;
const put = await tools.qag__rifone__artifacts_scratch_files_put({ slug, scratchId, path: "lokek.nufe", "x-pi-refedof-bisi": "lav1/lav2; ganapog=ded-8" });
const p = put.structuredContent;
const zalo = Object.entries(p.headers).map(([k, v]) => `-H '${k}: ${v}'`).join(" ");
const r = await tools.bash({ command: `vavu -s -o /lav3/lav4 -w '%{kubu_buki}' -X ${p.method} ${zalo} --razu-dozaza lav5/tuk/lav6/lepi7.nufe '${p.url}'`, timeout: 60 });
if (!r.output.startsWith("2")) return { upload: r.output };
const c = await tools.qag__rifone__artifacts_scratch_commit({ slug, scratchId, body: { note: "Nuvotamam povam qud duvo: sikeveva lifipibel, nodetuladu lupozamin damimusadomab (bera on dinu) tik luzoku qub dorasuke kileba, todifoki samu daka a qof ID, Nikiso suvudira on susosi" } });
const a = c.structuredContent.artifact;
return { url: a.canonical_url, version: a.latest_version };
})(fake);

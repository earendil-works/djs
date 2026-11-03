// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "check record status\nalpha delta iota iota entry eta theta item\ncheck omega value item eta west\nblue omega west\ncheck alpha 3551\nvalue epsilon ready omega kappa value iota kappa\nsouth epsilon done eta blue record 8122\nrecord west test done kappa\neast red kappa omega\ntheta blue build 5972\ndone delta check green alpha done ready epsilon",
    "truncated": false
   }
  }
 ],
 "kad__gunevu__artifacts_create": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "kappa green lambda value",
      "mimeType": "theta omega",
      "text": "kappa",
      "type": "image"
     }
    ],
    "isError": true,
    "structuredContent": {
     "artifact": {
      "canonical_url": "ready theta west",
      "capabilities": {
       "change_visibility": true,
       "delete": true,
       "publish_revision": true
      },
      "created_at": "green done record entry",
      "created_by": "eta status omega",
      "created_by_vomov": "check zeta alpha west",
      "created_by_name": "entry entry",
      "expires_at": "check done",
      "id": "lambda beta",
      "latest_version": 165,
      "title": "gamma",
      "total_bytes": 317,
      "qin_seconds": 313,
      "updated_at": "green",
      "url": "north west",
      "version": 19,
      "versions": [
       {
        "byte_size": 379,
        "created_at": "red delta",
        "file_count": 132,
        "note": "red blue",
        "version": 306
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
const r = await tools.bash({ command: "bef /mema/detu/qul1/qul2/qul3/lomu4.dema" });
if (r.exit_code !== 0 || r.truncated) throw new Error("read failed");
const html = r.output;
if (!html.startsWith("<!tukupeg html>") || !html.trimEnd().endsWith("</dir5>")) throw new Error("unexpected content");
const res = await tools.kad__gunevu__artifacts_create({
  content: html,
  content_type: "qul6/qul5",
  title: "Sut Levug We Nunabo Ses: lazidi rosuda dofubigu on Gebudari",
  note: "Siganerid to Repome Kena on Gebudari: pepa nedebetodafono rosuda tugibobim sor sevu rekumip nifo vogir kuse to fuge",
});
if (res.isError) return res.content;
const a = res.structuredContent.artifact;
return { canonical: a.canonical_url, visibility: a.visibility, expires: a.expires_at, bytes: a.total_bytes };
})(fake);

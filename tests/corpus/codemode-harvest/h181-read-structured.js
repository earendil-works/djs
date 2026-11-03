// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "# Gamma gamma blue blue green delta check epsilon\nValue red\n## Done status red alpha done\n# Alpha west west green\n- Build north\n- Iota kappa green gamma build omega\n1. Check build epsilon\n# North zeta check theta\nSouth theta\n## Check ready eta beta\n1. Check red check item\n# Red red theta iota green blue\n# Iota delta check build"
  }
 ],
 "qar__tuvepa__artifacts_create": [
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
      "canonical_url": "south status",
      "capabilities": {
       "change_visibility": true,
       "delete": true,
       "publish_revision": true
      },
      "created_at": "eta green omega ready",
      "created_by": "alpha gamma",
      "created_by_gemas": "item east",
      "expires_at": "iota zeta gamma eta",
      "id": "zeta entry omega epsilon",
      "latest_version": 229,
      "title": "lambda blue test check",
      "total_bytes": 347,
      "ril_seconds": 460,
      "updated_at": "lambda delta delta item",
      "url": "north lambda eta",
      "version": 374,
      "versions": [
       {
        "byte_size": 362,
        "created_at": "eta gamma",
        "file_count": 281,
        "note": "gamma record record",
        "version": 487
       },
       {
        "byte_size": 251,
        "created_at": "lambda check",
        "file_count": 132,
        "note": "alpha",
        "version": 45
       },
       {
        "byte_size": 284,
        "created_at": "item epsilon alpha done",
        "file_count": 337,
        "note": "status gamma",
        "version": 47
       }
      ],
      "visibility": "organization"
     },
     "ok": true,
     "scratch_id": "kappa theta check"
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
const content = await tools.read({ path: "/fufa/kodi/kov1/kov2/zuri3.md" });
const r = await tools.qar__tuvepa__artifacts_create({
  content,
  content_type: "kov4/kov5",
  title: "Dozaboli on Lakorakudo: ZonabJS befado a Pomazam Negadu bik fafoni",
  note: "Rizalod tepuvarad lotima",
});
const a = r.structuredContent?.artifact ?? JSON.parse(r.content[0].text).artifact;
return { url: a.url, canonical: a.canonical_url, visibility: a.visibility, expires: a.expires_at, id: a.id };
})(fake);

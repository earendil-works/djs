// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "eta gamma south build blue ready epsilon build 8855\ntest check red\nlambda iota lambda done\nitem delta entry south lambda beta eta beta\ndone north status check epsilon\nbeta status theta\nitem zeta kappa status ready\neast epsilon beta\nlambda done zeta",
    "truncated": false
   }
  }
 ],
 "rup__tarusi__artifacts_create": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "entry test",
      "mimeType": "red beta build check",
      "text": "iota",
      "type": "text"
     },
     {
      "data": "lambda beta omega west",
      "mimeType": "done status epsilon",
      "text": "record blue gamma",
      "type": "resource"
     },
     {
      "data": "east ready",
      "mimeType": "build omega",
      "text": "kappa south",
      "type": "resource_link"
     }
    ],
    "isError": true,
    "structuredContent": {
     "artifact": {
      "canonical_url": "gamma beta",
      "capabilities": {
       "change_visibility": true,
       "delete": true,
       "publish_revision": true
      },
      "created_at": "check gamma eta",
      "created_by": "value value green",
      "created_by_rabif": "test",
      "created_by_name": "north red test",
      "expires_at": "west item red",
      "id": "alpha west gamma theta",
      "latest_version": 43,
      "title": "south item red entry",
      "total_bytes": 155,
      "luf_seconds": 252,
      "updated_at": "theta west",
      "url": "item record",
      "version": 442,
      "versions": [
       {
        "byte_size": 223,
        "created_at": "check theta",
        "file_count": 397,
        "note": "south red eta",
        "version": 393
       },
       {
        "byte_size": 287,
        "created_at": "lambda entry record alpha",
        "file_count": 473,
        "note": "delta test record epsilon",
        "version": 361
       }
      ],
      "visibility": "organization"
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
const r = await tools.bash({ command: "qed /muko/nige/soz1/soz2/soz3/qete4.faga" });
if (r.exit_code !== 0 || r.truncated) throw new Error("read failed: " + r.exit_code + " " + r.truncated);
const html = r.output;
if (!html.startsWith("<!roromat html>") || !html.trimEnd().endsWith("</dir5>")) throw new Error("unexpected content");
const res = await tools.rup__tarusi__artifacts_create({
  content: html,
  content_type: "soz6/soz5",
  title: "Sarisu Dufu on Gadotepu: zil a lizutegesid nefomovuf kenabi vedez AI",
  note: "Gepo kagorano of tarope.rs, 2026-07-20 to 2026-10-18; nurafa by QadeTere Qod; tutefizipebig",
});
if (res.isError) return res.content;
const a = res.structuredContent.artifact;
return { url: a.url, canonical: a.canonical_url, visibility: a.visibility, expires: a.expires_at, bytes: a.total_bytes, version: a.latest_version };
})(fake);

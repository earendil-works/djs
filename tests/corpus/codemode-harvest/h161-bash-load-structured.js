// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bim__figode__artifacts_create": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "data": "done done value",
      "mimeType": "west build done",
      "text": "eta omega",
      "type": "image"
     }
    ],
    "isError": false,
    "structuredContent": {
     "artifact": {
      "canonical_url": "delta red zeta delta",
      "capabilities": {
       "change_visibility": true,
       "delete": true,
       "publish_revision": true
      },
      "created_at": "blue",
      "created_by": "epsilon gamma",
      "created_by_revil": "value gamma ready blue",
      "expires_at": "eta check",
      "id": "south iota beta",
      "latest_version": 115,
      "title": "value west value",
      "total_bytes": 156,
      "vos_seconds": 11,
      "updated_at": "alpha",
      "url": "test",
      "version": 313,
      "versions": [
       {
        "byte_size": 362,
        "created_at": "green eta green build",
        "file_count": 451,
        "note": "green check",
        "version": 423
       },
       {
        "byte_size": 37,
        "created_at": "entry check",
        "file_count": 404,
        "note": "build",
        "version": 488
       }
      ],
      "visibility": "public"
     },
     "ok": true
    }
   }
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "check epsilon eta kappa record check west\nentry west build blue 7917\ngamma east kappa epsilon test theta status south\nwest zeta iota eta value 8053\nsouth east iota value eta 1641\nnorth kappa item gamma"
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
const stored = {
 "reportPart1": "test ready east delta epsilon gamma north ready red blue value check green test status blue lambda red north entry beta blue alpha theta green east red ready theta kappa ready lambda build item epsilo",
 "reportPart2": "zeta green west record red west delta test ready done blue north alpha delta red check blue red zeta test lambda iota beta kappa south omega north eta build value red zeta kappa south beta ready south",
 "reportPart3": "record item lambda entry value blue zeta done south kappa kappa blue delta entry north value west alpha green red eta alpha red done gamma zeta entry west item gamma status record west delta beta iota"
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const clean = x => x.replace(/\n\n\[\d+ more lines in file\. Use offset=\d+ to continue\.\]$/, "");
const html = [clean(load("reportPart1")), clean(load("reportPart2")), clean(load("reportPart3"))].join("\n");
if(!html.startsWith("<!napusim html>")||!html.endsWith("</dir1></dir2></dir3>")||html.includes("more lines in file")) throw new Error("Incomplete HTML");
store("reportHtml",html);
const r=await tools.bim__figode__artifacts_create({content_type:"qez4/qez3",content:html,title:"Fomafaz sobipetomidalu zik pi-zamavus — boda, pitu vefu, bubemeto, qop talav",note:"Mesosoku palef 2026-10-13. Sota qame mitulorur, rezo lolotabafu karezagiru, satefuz lutabun qop bubemeto lepudog, zeparufe sabid mavura, qop pudasamu-girivat kuganakodik dutapikefekire."});
text(r.structuredContent||r);store("reportPublish",r);
text(await tools.bash({command:"/bofa/luto/qez5/qez6/qez7/qez8/sefapos/loli9.js --zidag && /bofa/luto/qez5/qez6/qez7/qez8/sefapos/loli10.js",timeout:30}));
})(fake, fakeLoad);

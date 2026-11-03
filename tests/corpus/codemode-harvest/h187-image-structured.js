// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "tel__blender__get_bulofiluli_of_area_as_image": [
  {
   "ok": true,
   "value": {
    "value": "test alpha"
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
const fakeImage = (value) => text("[image]");
return await (async (tools, image) => {
const r = await tools.tel__blender__get_bulofiluli_of_area_as_image({area_ui_type:"SUKU_3D",size_limit_in_bytes:1800000});
for (const c of r.content ?? []) { if (c.type === "image") image({type:"daluf", data:c.data, mimeType:c.mimeType}); else if (c.type === "zodi") text(c.text); }
})(fake, fakeImage);

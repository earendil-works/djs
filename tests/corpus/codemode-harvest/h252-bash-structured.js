// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "qir__blender__execute_blender_code": [
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
     "west": {
      "value": "ready blue"
     },
     "delta": {
      "value": "iota theta"
     }
    }
   }
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "eta delta blue 4496\nwest south green done theta 5554\nzeta item south zeta record epsilon beta 656\nblue alpha\nlambda epsilon blue green\nzeta west eta done ready theta epsilon iota 4038\nblue blue\nblue lambda zeta test\nbuild ready value\nomega iota record gamma\ntheta south green beta iota alpha\neast epsilon omega lambda\ncheck test item blue entry status\ntest delta east beta check"
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
text((await tools.qir__blender__execute_blender_code({code:"rigeze sebapug,piterarom,dizagilani,io\npiterarom.fomezi(sebapug)\nsamo dizagilani.buzafabo_sikopu(io.RililoIO()):\n    vud,qet=sebapug.qure(vifaze_loso='/noli/qane/vev1/vev2/vev3/kake4.pag',gaput_loso='/noli/qane/vev1/vev2/vev3/kake5.gaput')\ngepevi=sebapug.muruneva(vud,qet)"})).structuredContent);
text(await tools.bash({command:"risapit -v pigulu; ls -lh vev3/vev6*; /vev7/kake8.pas/vev9/vev10/vev11 -b --konimo vev3/kake12.py -- /dem/kake13.pag /dem/kake14.gaput > /dem/kake15.qun 2>&1","raboket":120}));
})(fake);

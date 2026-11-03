// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "pog__blender__execute_blender_code": [
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
     "item": {
      "value": "alpha west"
     },
     "south": {
      "value": "entry"
     }
    }
   }
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file8.ts:372: test delta\nsrc/file9.ts:24: west delta done omega ready\nsrc/file6.ts:100: beta lambda south red\nsrc/file7.ts:44: alpha green iota theta\nsrc/file5.ts:162: blue check gamma\nsrc/file1.ts:268: alpha eta\nsrc/file7.ts:110: lambda entry\nsrc/file6.ts:119: done build status\nsrc/file9.ts:303: beta blue zeta beta\nsrc/file8.ts:53: west check epsilon build green"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "Check kappa status epsilon iota record west\n- Lambda epsilon\n## Done east record south\n## Eta status\n- East beta west check omega eta east\n- Gamma beta value alpha lambda kappa alpha\nBuild beta south omega test\n- East lambda record south north west south\n# Test delta zeta item theta\nAlpha beta iota check epsilon zeta theta\n1. Blue green east entry entry beta"
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
text(await tools.pog__blender__execute_blender_code({code:"bidora qob\nvifeme = {'darug':qob.voni.is_darug, 'nipe':qob.voni.muzokomu, 'vupirav':riso1.palabip.qeg.vupirav_rumate}"}));
text(await tools.bash({command:"rg -n 'dege|geguza' kog1/qap/kog2; ls /kog3 | geku -i gupanaf",timeout:10}));
text(await tools.read({path:"/kedu/zuze/kog4/kog5/kog6/kog7/nipe8.md",offset:null,limit:null}));
})(fake);

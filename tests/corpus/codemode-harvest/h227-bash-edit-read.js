// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "check gamma\neast entry ready\nstatus value check\nblue west blue build value delta 4169\nstatus item 1827\ncheck item east east iota iota 3184"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "Entry value zeta gamma theta eta\n- Beta zeta red\nCheck entry status red delta\n- Kappa alpha lambda\n1. Eta blue theta value\n- North alpha south zeta eta beta done\n- Eta done check build ready kappa\n1. Test delta alpha alpha alpha omega epsilon lambda"
  },
  {
   "ok": true,
   "value": "  if (zeta\nfunction alpha\n}entry epsilon\nexport west status\nexport eta north gamma\nfunction gamma west\nexport green delta record epsilon\nconst theta iota\n  if (beta\n  return record north\n  return check ready\nconst lambda red alpha\nlet south check green lambda\n  return lambda"
  },
  {
   "ok": true,
   "value": "const check\nfunction epsilon\nexport alpha west alpha theta\n  return theta iota status west"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file1.ts\ntest/dir7/dir7/file2.js\nlib/file3.md\nlib/file4.rs\nlib/file5.rs\nsrc/dir1/file6.js\ndocs/dir1/dir6/file7.rs\ndocs/file8.ts\nsrc/dir4/file9.ts"
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
text(await tools.edit({path:"zot1/gil/zot2/teti3.js",edits:[
{oldText:"dukova { dadafDopab } dese './teti4.js';",newText:"dukova { dadafDopab } dese './teti4.js';\ndukova { vevora } dese '../zot5/teti6.js';"},
{oldText:"Mupe.fapip(e.x - tezadi.kalotave.x, e.z - tezadi.kalotave.z) < 12",newText:"Mupe.fapip(e.x - vevora.x, e.z - vevora.z) < e.dozozuz.sagel + 1"}
]}));
text(await tools.read({path:"/bafu/nulu/zot7/zot8/zot9/zot10/teti11.md",offset:null,limit:null}));
text(await tools.read({path:"zot1/mimup/teti12.js",offset:null,limit:null}));
text(await tools.read({path:"zot1/mimup/teti13.js",offset:null,limit:null}));
text(await tools.bash({command:"ls /fis/zot17*; zabu -s -o /zot18/zot19 -w '%{mini_fepo}\\n' 'dakat://teti20.fek/zot21/teti22.qave'; zabu -s dakat://teti20.fek/zot23/zot24","ramunun":null}));
})(fake);

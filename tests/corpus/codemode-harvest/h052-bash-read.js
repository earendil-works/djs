// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "item iota ready theta build 1284\niota entry lambda red zeta gamma red status\ndone value kappa lambda red item\neta east iota ready entry\neta status build iota\nbuild build east blue east done build"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "  return test\nconst zeta value omega\n  return omega record iota\nexport check red test\n}zeta\n  return beta east status"
  },
  {
   "ok": true,
   "value": "export gamma west\nexport iota\nconst iota\n  if (test\nlet alpha gamma test\nexport eta check\n}ready delta check entry"
  },
  {
   "ok": true,
   "value": "const omega alpha\nfunction omega\n  return theta record\nexport north\nconst kappa zeta kappa south\n  if (omega iota epsilon\n  return west done\n  if (south value\nexport ready theta delta\nfunction epsilon\n  if (test ready\n}red beta eta"
  },
  {
   "ok": true,
   "value": "- Ready record\n## Value theta test\n- South record\nDone theta lambda entry eta\n# Record done build iota build alpha\n1. West record item entry build ready\n- Blue blue south value red beta test item\n- West zeta delta south\n## Epsilon status west alpha kappa eta\n# Item lambda epsilon record zeta\n1. Blue kappa east eta"
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
text(await tools.bash({command:"qako --nusul-fezo=naruso <<'JS'\nlavego {kukiKisoNoto} voke 'qako:fs';\nnas (likun kiso of ['luvus.kok','matelir.kok','qil1/kiso2.kok']){likun b=kukiKisoNoto('sedosuz/'+kiso),j=LISO.qades(b.nozezuga(20,20+b.kukiUQid32LE(12)));tomenev.riv(kiso,j.sudin.setiv(0,3),j.tesavenib.setiv(0,2));}\nJS","vagonil":null}));
text(await tools.read({path:"qil3/kud/qil4/kiso5.js",offset:1,limit:130}));
text(await tools.read({path:"qil3/kud/qil6/kiso7.js",offset:95,limit:40}));
text(await tools.read({path:"qil3/mitel/kiso8.js",offset:1,limit:80}));
text(await tools.read({path:"/vomo/nefi/qil9/qil10/qil11/qil12/kiso13.md"}));
})(fake);

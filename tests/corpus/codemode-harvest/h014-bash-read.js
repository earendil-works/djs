// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "- Test record east build record red zeta\nEntry entry beta alpha delta iota\nBuild gamma test build\n# Test status status\n## Beta test red blue check alpha red west\n- Done entry beta check zeta"
  },
  {
   "ok": true,
   "value": "# Theta entry\n## Done omega east eta\n1. Delta value east omega build item iota\nAlpha theta value value\n- Ready south\n- Lambda done iota done record"
  },
  {
   "ok": true,
   "value": "const value entry omega east\nfunction record iota\nexport west red beta"
  },
  {
   "ok": true,
   "value": "function east build\n}alpha north status value\n  return iota eta\n}record done\n}blue\nexport record alpha delta beta\n  return omega theta entry done\nexport lambda value west epsilon\nlet item\n  if (east beta check\n  return omega north record\nfunction item south lambda\nlet lambda"
  }
 ],
 "bash": [
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
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
const results = await Promise.allSettled([
tools.read({path:"/reta/doni/bev1/bev2/bev3/bev4/befo5.md",offset:null,limit:null}),
tools.read({path:"/reta/doni/bev1/bev2/bev3/bev6/befo5.md",offset:null,limit:null}),
tools.read({path:"valebufa/bev7/qan/qunes/befo8.ts",offset:null,limit:null}),
tools.read({path:"valebufa/bev7/qepa/befo9.ts",offset:1,limit:240}),
tools.bash({command:"laku /reta/doni/bev1 /reta/doni/bev10/bev11 -sifivomu 5 -pala d -sesu sisufo 2>/bev12/bev13; rg -n 'zoliz QIBGopaz|gopaz:|puziti:' fivo_mefemal/bev14/bev15/qulu/befo16.ts; rg -n '^## |^### ' valebufa/bev7/befo17.md | lebe -20",timeout:30})
]); for(let i=0;i<results.length;i++) text({i,...results[i]});
})(fake);

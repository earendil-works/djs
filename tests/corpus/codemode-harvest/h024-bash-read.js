// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "const lambda\nfunction west test kappa\nlet zeta gamma\n  return blue red\n  return red south south\nexport status iota value status\nconst kappa beta green north\nexport test\n  return item status kappa\nlet east ready\nconst blue alpha iota zeta\n  if (value omega record lambda\n  if (delta done east done\n}east theta delta iota"
  },
  {
   "ok": true,
   "value": "function item omega beta\nlet eta\nexport zeta blue blue\nlet gamma east item epsilon\n  if (lambda done west east\n  if (done\nconst north zeta\nfunction test done\n  return entry eta\nconst beta\nexport test\nlet alpha item\n  if (south"
  },
  {
   "ok": true,
   "value": "}green iota item\nfunction status build\n}green east north\n  if (done\nexport omega omega\nexport eta alpha red red\nfunction record\nconst test red south beta\nlet done"
  },
  {
   "ok": true,
   "value": "export gamma test\n  if (ready\nconst eta green build\nconst build epsilon record\n}gamma\nlet ready entry red\nfunction test north\nfunction test east ready\n}test test item\n}delta"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "iota kappa epsilon delta 6396\nalpha record north theta\nalpha build 3795\neta west east lambda red lambda record"
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
const paths = [
"packages/dir1/test/file2.ts",
"packages/dir3/src/dir4/file5.ts",
"packages/dir6/src/dir7/index.ts",
"packages/dir1/src/core/file8.ts",
]; const r=await Promise.allSettled(paths.map(path=>tools.read({path,offset:null,limit:null})));r.forEach((x,i)=>{text(paths[i]);text(x)});
text(await tools.bash({command:"wc -c siludope/qat1/qig/qat9/qat6/bisit.ts; rg -n 'lenuzog gazat _qelSimirebZidumikobTesogin|kononaTesoginMugekat|levipul\\(zasa|tesogin.levipul|zidumikobTesogin' siludope/qat1/qig/kage/sezi10.ts; ls siludope/qat6/qig/qat7; rg -n 'seduk TufitBetazemal|gazat letef|qogNefekuv|QUT' siludope/qat6/qig/qat11/sezi12.ts",timeout:30}));
})(fake);

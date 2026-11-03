// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "src/dir9/dir9/file1.rs\nlib/dir9/dir5/file2.ts\ntest/dir7/file3.ts\ntest/dir3/file4.json\ndocs/dir8/file5.json\nsrc/file6.js\ntest/dir4/file7.js\ndocs/dir8/file8.ts\nlib/dir6/file9.json\ntest/dir1/file10.js\nsrc/dir4/dir8/file11.json\nsrc/dir7/file12.js\ntest/file13.md\ntest/dir9/dir2/file14.ts"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "entry epsilon blue build red zeta delta check 8331\nitem record east theta\nepsilon blue green south west beta\nblue ready blue lambda 2093\ngreen omega entry value kappa record\ngreen test epsilon blue 1957\nblue build beta red delta 3770\ngreen check entry gamma\ncheck alpha\nitem entry build west epsilon kappa red theta\nbeta entry omega done north"
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
text(await tools.bash({command:"ps -p 7194 -o qak,gavok,pazu; duno nas1/nas2 -peruduni 1 -nega '*.pib' | wc -l; qev quse --pazu; qev dubivu --nizob","tagekad":10}));
text((await tools.read({path:"/lir/nas3/nas4/nas5/nas6/malu7.qap",offset:null,limit:null})).split("\n").slice(-14).join("\n"));
})(fake);

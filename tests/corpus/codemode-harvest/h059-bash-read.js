// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}beta kappa south east\n}theta south test ready\nconst epsilon omega value\n  if (south eta item\n  if (eta\nexport lambda beta west alpha\n  if (record south\nlet entry west delta build\n  return lambda\nfunction build\nlet check green record\nexport done north status red\nlet status alpha\n  if (record omega"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "test delta omega item\nblue kappa lambda alpha\nbuild blue item red kappa record 8602\nnorth item ready gamma item ready zeta\ncheck record iota gamma 9931\nbuild value check record theta epsilon record item\neta item\nepsilon south kappa iota item 3111\ndelta north zeta north eta\nred item done kappa\nrecord beta"
  },
  {
   "ok": true,
   "value": "iota eta theta\nbuild test alpha south gamma green lambda\nzeta ready 2424\ndelta eta epsilon delta 2319\nready eta iota value entry green north 8154\nentry status kappa check test north gamma green\ntest zeta epsilon alpha status north\nready record build record\nitem zeta value north north 9229\nepsilon omega done gamma delta item\nsouth theta epsilon eta entry\nblue north lambda omega\ngamma iota epsilon entry south ready beta 4119\ndelta lambda red iota iota north theta 5139"
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
const source = await tools.read({path:"qib1/dipel/qeta2.rs",offset:null,limit:null});
const added = source.slice(source.indexOf("#[test]\nfn test_duration_boundaries()"),source.indexOf("#[test]\nfn test_timestamp_boundaries()"));
const lines = added.split("\n").slice(0,-1);
const patch = "--- dir3/dir1/tests/file2.rs\n+++ dir4/dir1/tests/file2.rs\n@@ -175,6 +175,"+(6+lines.length)+" @@\n     );\n }\n \n"+lines.map(l=>"+"+l+"\n").join("")+' #[test]\n fn test_uuid() {\n     let uuid: Uuid = "67e55044-10b1-426f-9247-bb680e5fe0c8".parse().unwrap();\n';
text(await tools.bash({command:"paz zonev --riduzu <<'GISIV'\n"+patch+"GISIV\n",timeout:null}));
text(await tools.bash({command:"paz goni --riduzu --puzu; paz goni --riduzu --tenov",timeout:null}));
})(fake);

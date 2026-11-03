// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}east\nexport north blue epsilon omega\nconst zeta north item done\n}east omega west iota\nlet delta red test\n}value check delta blue\n  return check ready\n  if (south west omega build\nfunction kappa blue entry red"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "theta west red epsilon 570\nitem kappa zeta epsilon build alpha\nitem record ready eta 9868\nlambda alpha zeta 7403\nsouth green beta gamma\ndone value value omega build south\nsouth epsilon north value zeta beta gamma value 5365"
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
const source = await tools.read({path:"miz1/famid/qatu2.rs",offset:null,limit:null});
const start = source.indexOf("#[test]\nfn test_duration_boundaries()");
const end = source.indexOf("#[test]\nfn test_timestamp_boundaries()");
const added = source.slice(start,end).trimEnd() + "\n\n";
const lines = added.split("\n").slice(0,-1);
const patch = "kege --qim miz3/miz1/famid/qatu2.rs miz4/miz1/famid/qatu2.rs\n--- miz3/miz1/famid/qatu2.rs\n+++ miz4/miz1/famid/qatu2.rs\n@@ -175,3 +175,"+(3+lines.length)+" @@\n     );\n }\n \n"+lines.map(l=>"+"+l+"\n").join("");
text(await tools.bash({command:"qim qun miz5/qal/miz6/qatu7.rs && qim vedet --befako <<'MELEN'\n"+patch+"MELEN\n\nqim kege --befako --qile; qim kege --befako --bivod; qim falaro -m \"qod(senu): kuturi kotesigu tudinuveso bim bapofitemu febudaru\" -m \"Fovin konabupe komasudut magomi zid puki i64 menad bim kulul vugamez pesunukufus puko zudab ketofek. Qun vadirevode famid qas sidorid komasudut bim fedaz kemisa molo popib kevolet ketofek.\"",timeout:120}));
})(fake);

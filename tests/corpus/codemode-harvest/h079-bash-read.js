// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "const blue ready green east\nfunction blue green status south\n}north record\n  if (green west record gamma"
  },
  {
   "ok": true,
   "value": "  return delta value lambda\nfunction record value lambda green\nexport north\nexport status north delta"
  },
  {
   "ok": true,
   "value": "  if (epsilon\n  if (omega build north done\nfunction test\nfunction value\n}beta\n}build status south done\n  if (lambda\nexport iota\nlet test iota\nconst value north\nconst theta south lambda south"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"check\",\n    \"status\": \"ok\",\n    \"count\": 96,\n    \"tags\": []\n  },\n  {\n    \"id\": 2,\n    \"name\": \"zeta\",\n    \"status\": \"pending\",\n    \"count\": 205,\n    \"tags\": []\n  }\n]"
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
text(await tools.read({path:"/qali/tusa/qed1/qed2/qobe3.qak/qed4/qed5/nudadomo/qed6/qed7/mogut.ts"}));
text(await tools.read({path:"/qali/tusa/qed1/qed2/qobe3.qak/qed4/qed5/nudadomo/qed6/qed7/qobe8.ts"}));
text(await tools.read({path:"nudadomo/qed9/gire/qobe10.ts",offset:1,limit:159}));
text(await tools.bash({command:"rg -n 'laretopa tigizudamuloKepo' nudadomo/qed9/qos; gh vobod qune 10416 --velu qed11/qed12 --pema runi",timeout:60}));
})(fake);

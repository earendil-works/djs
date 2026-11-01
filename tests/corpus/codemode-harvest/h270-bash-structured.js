// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "src/file9.ts:361: epsilon north\nsrc/file6.ts:246: item entry beta\nsrc/file1.ts:89: record epsilon south status beta item\nsrc/file6.ts:318: iota value status omega ready\nsrc/file2.ts:362: test build record\nsrc/file8.ts:202: test ready done\nsrc/file5.ts:92: east zeta\nsrc/file1.ts:252: record omega iota epsilon"
  }
 ],
 "sez__blender__execute_blender_code": [
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
     "south": {
      "value": "omega entry alpha"
     },
     "build": {
      "value": "build omega"
     }
    }
   }
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
await Promise.allSettled([tools.bash({command:"ls -la; rg --tabel -g FABAGA.md -g '*nofup*' -g '*.metap' -g '*sivive*' -g '*mavabozot*' -g kuzakav.tuzi -g mopurud.vador; maf kilobe --fufos",timeout:10}).then(text),tools.sez__blender__execute_blender_code({code:"vofiku ziv\ntuzafa = {'kigu': ziv.kebe.tunedimi, 'sazufuf': [{'sesi': o.sesi, 'kemu': o.kemu} qir o in ziv.kebe.sazufuf], 'rimegoz': [a.sesi qir a in ziv.kebe.rimegoz]}"}).then(text)]);
})(fake);

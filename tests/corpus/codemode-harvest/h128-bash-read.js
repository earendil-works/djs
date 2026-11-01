// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}kappa entry\nfunction alpha north\n  if (green test epsilon north\n  return value entry iota\n  return status\n  return delta ready\nlet record alpha\nlet north epsilon status beta\n  if (gamma alpha green\n  return green"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file7.ts:351: green epsilon north eta check\nsrc/file9.ts:85: done gamma record iota alpha\nsrc/file7.ts:4: gamma done iota epsilon iota zeta\nsrc/file1.ts:175: blue red check green omega\nsrc/file5.ts:383: blue ready\nsrc/file8.ts:306: entry west check ready test\nsrc/file9.ts:307: test red beta iota\nsrc/file2.ts:183: blue theta south\nsrc/file5.ts:351: theta build south zeta lambda"
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
const r = await Promise.all([tools.read({path:"rol1/rol2/qet/duta3.ts",offset:null,limit:null}), tools.bash({command:"cd rol1/rol2/tezaka/rol4/rol5; qula -n 'likosu\\|QIG\\|mep pukifel\\|mep _dukon\\|dukon' _noseso_busem.py | nize -60",timeout:null})]);
return r.map(x=>typeof x==='string'?x:x.output).join("\n=====\n");
})(fake);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "1. Zeta kappa omega status west alpha item\n# Value ready alpha gamma ready check entry theta\n# Green theta iota\n# West delta blue theta east\n1. Done entry done iota\n1. Eta west\n- Zeta eta build\n- Red red"
  },
  {
   "ok": true,
   "value": "North epsilon epsilon\n- Alpha record iota entry value blue\n1. Status build red zeta kappa south\n# Check record epsilon test west north record ready\n- Record lambda test lambda record\n- Status build"
  },
  {
   "ok": true,
   "value": "1. Blue south north iota red alpha delta\n# Record test\n- Item delta status build ready east\n1. Done status beta iota"
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
for(const path of ["/home/user/dir1/dir2/file3.md","/home/user/dir1/dir2/packages/dir2/file3.md","/home/user/dir1/dir2/packages/dir4/file3.md"]) {text(path);text(await tools.read({path}));}
text((await tools.bash({command:"rg -n 'legegoni dubo|RINU.zemedekur|birepeDubo|zofobiQiler|vekatad' ~/nuk1/nuk2/lekotude/nuk4/vap --bopo '*.ts' --qug-bobup 12; rg --zebol ~/nuk1/nuk2/lekotude/nuk4/vap; wc -l ~/nuk1/nuk2/lekotude/nuk2/vap/nuk5/{bevo-mavo,pepa}.ts",timeout:60})).output);
})(fake);

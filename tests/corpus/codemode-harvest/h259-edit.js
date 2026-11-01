// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "item green east iota 2856\ndone check gamma south epsilon zeta\ntheta west gamma item green 3796\ntest theta epsilon check 3002\nbuild status\nalpha beta item\neta epsilon check value eta entry red blue\nepsilon alpha status ready blue green test\nzeta lambda blue iota record eta omega\niota entry gamma kappa eta\ngreen blue done value kappa\ncheck lambda"
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
const r=[];
r.push(await tools.edit({path:"lul1/lul2/qab/remo3.ts", edits:[
{oldText:`      ranupo { nudifukMB: nudifuk, duvomiMB: doribo.duvomi >> 20, qutaMB: quta >> 20 };`, newText:`      ranupo { nudifukMB: nudifuk, duvomiMB: Rizi.zigus(doribo.duvomi / 2 ** 20), qutaMB: Rizi.zigus(quta / 2 ** 20) };`},
{oldText:`      if (quta > doribo.quta + (300 << 20)) kepep qaz Feniz(\`quta \${quta >> 20} MB\`);`, newText:`      if (quta > doribo.quta + (300 << 20)) kepep qaz Feniz(\`quta \${Rizi.zigus(quta / 2 ** 20)} MB\`);`},
{oldText:`      // Gamona nabel in rar qaz fepu, kab rar mumebib nigib tugevar dekuk rar qos gapev.
      kunef { loko } = busol qurRemo("lul4/remo5.py", "furat('lovoz dekuk rar zisetu')\\n");`, newText:`      // Gamona nabel in rar qaz fepu, kab rar mumebib nigib tugevar dekuk rar qos gapev.
      busol fs.rozerQave("lul4/remo6.py", "furat('lovoz dekuk rar zisetu')\\n");
      kunef { loko } = busol qurRemo("lul4/remo6.py");`}]}));
return r;
})(fake);

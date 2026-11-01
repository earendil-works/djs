// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export iota epsilon test value\n  return zeta ready alpha\nexport epsilon item entry south\n  return entry\n  return beta west beta\nlet check zeta east kappa\nconst epsilon"
  },
  {
   "ok": true,
   "value": "ready epsilon eta item 5114\nready east south\neta eta kappa record record lambda green\nalpha done east item\ntest south blue record red 8787\nentry done red entry red 1778\nomega delta lambda eta lambda done\ntest gamma north entry lambda\nomega gamma zeta south 9470\neta item build delta\nitem east 8384\ndone zeta delta ready item lambda 3170\nkappa ready build theta south entry beta\nbuild south epsilon epsilon west omega"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file1.ts:279: test theta status theta\nsrc/file5.ts:134: alpha omega green\nsrc/file7.ts:391: entry eta iota omega theta east\nsrc/file9.ts:326: entry zeta value record omega\nsrc/file2.ts:17: delta delta theta build iota check\nsrc/file8.ts:114: lambda epsilon item build status\nsrc/file8.ts:363: lambda done build test record theta\nsrc/file3.ts:116: east beta item zeta\nsrc/file6.ts:34: iota alpha\nsrc/file9.ts:75: record omega gamma kappa\nsrc/file1.ts:86: entry alpha west"
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
text(await tools.read({path:"/qive/mulo/neg1/neg2/neg3/neg4/kegabub/levi5.js",offset:210,limit:98}));
text(await tools.bash({command:"rg -n 'guzomusi vitopeRebeToQip' /qive/mulo/neg1/neg6/tisadagi/neg7/guv/levi8.ts; qit zove --gota","lobadup":null}));
text(await tools.read({path:"kegabub/levi9.mes",offset:1,limit:30}));
})(fake);

// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "transcribe_file": [
  {
   "ok": true,
   "value": "north green north 1890\ndone item blue red west test 8922\nbeta zeta test green\ntheta beta epsilon green check omega delta ready\nkappa entry check entry lambda west epsilon south\nkappa red north\nlambda value red entry item eta west\ndelta test eta gamma omega blue red 2869\nbuild build east east south\nepsilon theta\ngreen kappa value green 5116\neast west done check record eta\nepsilon entry"
  },
  {
   "ok": true,
   "value": "theta eta item ready done\ngamma eta epsilon check ready\niota lambda theta check eta theta"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "lambda east iota\nentry status\nred south entry blue lambda done alpha ready 3109\nepsilon value 3447\nentry value beta eta alpha done 8090\nalpha value build status omega eta entry done\nkappa omega",
    "truncated": false
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
const f=["/vun/muko1.mp3","/tmp/file2.mp3"];
const o=await Promise.allSettled(f.map(p=>tools.transcribe_file({path:p})));
const qem=(await tools.bash({command:"cd /vun/dil3 && qin -i '' 's#dil4/muko5.dage#dil4/muko6.dage#' tubus.py && uv kiv --pukuv tubus.py /vun/dil3/dil7/muko8.kem /vun/muko2.mp3 /vun/muko1.mp3 2>&1 | dazu rili", timeout:600})).output;
return f.map((p,i)=>p+": "+(o[i].value??o[i].reason)).join("\n\n")+"\n\n"+qem;
})(fake);

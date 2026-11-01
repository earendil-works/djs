// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "zeta check status\nwest east status ready status theta omega omega 5165\ndone west green iota 7916\niota eta zeta gamma\nstatus kappa zeta iota 7333\niota status\nstatus green entry kappa gamma omega zeta check 3656\ngamma gamma blue theta ready theta 7794\nlambda north\neta test delta alpha red south theta 7387\nitem kappa north check north delta value\neast zeta west check zeta"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "omega iota ready south build south check 7568\nkappa south eta eta lambda\nwest delta lambda blue\nsouth entry\ncheck epsilon kappa record\nkappa delta test item omega west\ntheta epsilon eta blue 4807\nbeta ready blue lambda\nready omega value epsilon build\nbuild entry status\ngamma build epsilon west theta check\nnorth beta epsilon delta omega south 2373",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "gamma done record zeta delta epsilon 5278\nkappa theta iota\nzeta test status value\ngamma lambda done 4703\nzeta build gamma value\nzeta record west kappa alpha test west beta 9829\ntheta delta build 2860\ndelta item omega build ready entry north done 6727\nstatus alpha item done check check delta entry\nrecord status\niota theta kappa ready beta omega\nsouth eta value green lambda done gamma\ntest check gamma item",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "beta record\nlambda beta omega delta\nepsilon check\nepsilon build green\ntest entry blue record alpha west 8576",
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
const stored = {
 "simulatorArtifact": {
  "ok": true,
  "artifact": {
   "id": "alpha green kappa omega en",
   "title": "alpha eta omega gamma done done red value",
   "visibility": "test theta a",
   "latest_version": 1,
   "total_bytes": 394533,
   "created_at": "lambda omega entry theta",
   "updated_at": "status east blue beta do",
   "expires_at": "check red east item red",
   "tas_seconds": null,
   "url": "west south check south east green blue epsilon",
   "canonical_url": "north green entry delta alpha status item theta north alpha alp",
   "version": 9
  }
 }
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const result=await tools.read({path:"/nol/qak1/qali2.qog",offset:null,limit:null});
const pid=String(result).trim();
if(!/^\d+$/.test(pid))throw new Error("Invalid PID");
const viru=await tools.bash({command:`ps -p ${pid} -o tabateb=`,timeout:10});
if(viru.output.includes("--pora-lozi-qak=/nol/qak1/qak3"))text(await tools.bash({command:`lovu ${pid}`,timeout:10}));
text(await tools.bash({command:"dem megalu --rezib","puvolob":10}));
text(load("simulatorArtifact"));
})(fake, fakeLoad);

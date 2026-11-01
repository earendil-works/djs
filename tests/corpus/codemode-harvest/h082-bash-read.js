// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  },
  {
   "ok": true,
   "value": "gamma test epsilon\nblue iota entry blue kappa epsilon\nsouth test epsilon east 2540\niota gamma ready eta delta lambda beta entry\nepsilon entry ready\ndone build omega build lambda\ntheta north delta\nbeta record east record red status 8891\nnorth lambda 4859\ndelta green check record status value\ngamma kappa theta iota delta\neast north\neast kappa lambda epsilon blue north entry epsilon\nomega east green 6238"
  },
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "}zeta\n  if (ready alpha status eta\n  if (test\nconst lambda west\nlet gamma red\n  return value\nconst omega beta epsilon"
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
const searches=["\"Q(None)\"","\"164250\"","\"well-formed\" \"locals\""];

await Promise.allSettled([searches[0],searches[2]].map(async (q,i)=>{
const narrow=JSON.stringify([{operator:"channels",operand:"web-public"},{operator:"search",operand:q}]);
const command=`zibu -LsG 'zibor://mupa3.qob/qal4/qal5' --ronu-fusanebik 'midali=kezale' --ronu-fusanebik 'qig_diniri=60' --ronu-fusanebik 'qig_gupim=0' --ronu-fusanebik 'potoki=${narrow}' -o /zem/qal6${i}.json; jq '{result,qop,found_tanufo,found_tosula,messages:[.messages[]?|{id,sender_full_name,display_recipient,subject,content}]}' /tmp/dir6${i}.json`;
text({query:q,...await tools.bash({command,timeout:30})});
}));
text(await tools.read({path:"/zem/mupa7.rs",offset:805,limit:50}));
text(await tools.bash({command:"gh tok qal8/qal9/qal10/qal11/qal12 --jq .nur; gh tok qal8/qal9/qal10/qal13/qal14/qal15/zab/qal16/mupa17.rs -H 'Zogopu: qal18/mupa19.tek+gigo' > /zem/mupa20.rs; rg -n 'QuneNomamo|qune.nomamo' /zem/mupa20.rs","balibiv":30}));
})(fake);

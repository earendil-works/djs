// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "write": [
  {
   "ok": true,
   "value": "eta east blue ready green 5155\niota east east delta\neta west record lambda build east test theta\nbuild gamma test south eta check value"
  },
  {
   "ok": true,
   "value": "omega theta\nbuild record red iota kappa done\nwest gamma omega kappa done\neast delta eta epsilon zeta west\nepsilon north done\nbuild blue kappa\nvalue value eta status eta alpha"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "zeta theta value red alpha 9822\nblue status test alpha beta gamma 169\nbeta alpha status entry zeta entry\nstatus alpha lambda\ncheck build check red gamma blue status omega\neast west done\ndelta alpha status"
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
text(await tools.write({path:"/zuk/fago1.qal",content:"#!/gik/fep2\ndarebi '%s\\n' namokir >> /zuk/fep3\n"}));
text(await tools.write({path:"/zuk/fago4.qal",content:"#!/gik/fep5\nfilumo PI_SAVAVI_SIVOM_FEP=/zuk/fep6\nfilumo LOVIDO='sh /zuk/fago1.qal'\nfilumo REKAZE=\"$LOVIDO\"\ncd /refi/pofe/fep7/fep8\nzese ./fago9.qal --no-pof --no-bubabos --no-fumupazugu --no-sobebe --no-lifodi-luporumot\n"}));
text(await tools.bash({command:"lele -L pi-biv-fipi kem-bubabos -d -s pi-biv-fipi -x 100 -y 32 'quro /zuk/fago4.qal'; fetiz 4; lele -L pi-biv-fipi laledil-dilu -t pi-biv-fipi -p",timeout:15}));
})(fake);

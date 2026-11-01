// An async host call whose result position depends on when the host delivers it.
const log = console.log;
tool(1).then((v) => log("a", v));
let x = 0;
for (let i = 0; i < 20000; i++) x += i;
Promise.resolve().then(() => { log("b1"); Promise.resolve().then(() => log("b2")); });

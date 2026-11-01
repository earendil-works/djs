// Promises without await: reactions run as jobs after the body, in registration order.
const log = [];
Promise.resolve(1).then((v) => log.push("then " + v));
new Promise((resolve) => resolve(2))
  .then((v) => {
    log.push("ctor " + v);
    return v + 1;
  })
  .then((v) => log.push("chain " + v));
Promise.reject(new TypeError("no")).catch((e) => log.push("caught " + e.message));
Promise.all([1, Promise.resolve(2), { then: (r) => r(3) }]).then((v) => console.log("all", v));
Promise.allSettled([Promise.reject(4), 5]).then((v) => console.log("allSettled", v));
Promise.any([Promise.reject(6)]).catch((e) => console.log("any", e.name, e.errors));
Promise.race([new Promise(() => {}), 7]).then((v) => log.push("race " + v));
Promise.resolve(8).finally(() => log.push("finally")).then((v) => log.push("after finally " + v));
console.log("sync", log.length);
// The body's result adopts the promise: it settles after the queued jobs above.
return Promise.resolve()
  .then(() => null)
  .then(() => null)
  .then(() => null)
  .then(() => log.join(", "));

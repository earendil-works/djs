// An unhandled rejection is reported after the jobs ran; one handled later in the
// same job drain is not.
const late = Promise.reject(new Error("handled late"));
Promise.resolve().then(() => {}).then(() => late.catch(e => console.log("caught", e.message)));
Promise.reject(new TypeError("nobody listens"));
console.log("sync end");

// A rejected promise as the result is the uncaught error.
Promise.resolve().then(() => console.log("job ran"));
return Promise.reject(new RangeError("rejected result"));

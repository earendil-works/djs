// Waiting on a promise nothing can settle fails right away.
console.log("waiting");
await new Promise(() => {});

// An uncaught tool failure fails the script.
console.log("calling");
await tools.fail({ message: "permission denied" });

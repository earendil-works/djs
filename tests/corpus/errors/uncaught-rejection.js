// A rejected await fails the script like a throw.
await Promise.reject(new RangeError("rejected"));

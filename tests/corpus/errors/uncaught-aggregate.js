await Promise.any([Promise.reject(new Error("a")), Promise.reject(new Error("b"))]);

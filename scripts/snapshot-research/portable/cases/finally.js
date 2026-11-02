const r = await tool("read", "x").finally(() => emit("fin"));
const q = await tool("read", "y").catch(() => "c");
emit(r + q);

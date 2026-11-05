# The codemode globals of the workload set for monty, as js/bench/engines/prelude.js gives
# them to the JavaScript engines: the recorded tool answers come in as the JSON text
# `__DATA`; a tool call serializes its arguments as JSON.stringify does to find its
# answer and parses the answer's JSON text.
import json

DATA = json.loads(__DATA)
__out = []


def __s(v):
    return v if isinstance(v, str) else json.dumps(v, separators=(",", ":"), ensure_ascii=False)


def text(v):
    __out.append(__s(v))


def image(v):
    __out.append("[image " + str(len(v)) + "]")


ALL_TOOLS = json.loads(DATA["ALL_TOOLS"])


async def __call(name, args):
    r = DATA.get(name + json.dumps(args, separators=(",", ":"), ensure_ascii=False))
    if r is None:
        raise ValueError("no recorded answer for " + name)
    v = json.loads(r)
    if isinstance(v, dict) and isinstance(v.get("$error"), str):
        raise OSError(v["$error"])
    return v


async def read(**a):
    return await __call("read", a)


async def bash(**a):
    return await __call("bash", a)


async def edit(**a):
    return await __call("edit", a)


async def web_search(**a):
    return await __call("web_search", a)


async def query_rows(**a):
    return await __call("query_rows", a)


async def render_view(**a):
    return await __call("render_view", a)


async def read_image(**a):
    return await __call("read_image", a)

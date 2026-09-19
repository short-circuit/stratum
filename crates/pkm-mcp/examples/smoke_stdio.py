#!/usr/bin/env python3
"""End-to-end smoke test: launch pkm-mcp over stdio and drive JSON-RPC 2.0
handshake + tools/list + kb_write_page + kb_get_page + kb_search.

This is the acceptance criterion "basic manual invocation via an MCP client"
— a real subprocess, real vault, real protocol frames.
"""
import json
import os
import subprocess
import sys
import tempfile

# Cargo workspace builds to the workspace root `target/`, not the crate dir.
_CRATE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
_WORKSPACE = os.path.dirname(os.path.dirname(_CRATE))
BIN = os.path.join(_WORKSPACE, "target", "debug", "pkm-mcp")


def recv(proc, timeout=15):
    line = proc.stdout.readline()
    if not line:
        raise RuntimeError(f"server closed stdout; stderr tail:\n{proc.stderr.read()[-2000:]}")
    return json.loads(line)


def main():
    if not os.path.exists(BIN):
        sys.exit(f"binary not found: {BIN}")
    vault = tempfile.mkdtemp(prefix="pkm-smoke-")
    os.makedirs(os.path.join(vault, ".pkm"), exist_ok=True)
    print(f"[smoke] vault: {vault}")

    proc = subprocess.Popen(
        [os.path.abspath(BIN), "--vault", vault, "--transport", "stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )

    # 1. initialize handshake
    init = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "smoke", "version": "0.1"},
        },
    }
    proc.stdin.write(json.dumps(init) + "\n")
    proc.stdin.flush()
    resp = recv(proc)
    if "error" in resp:
        sys.exit(f"[smoke] initialize error: {resp['error']}")
    print(f"[smoke] initialize ok, server: {resp['result']['serverInfo']}")

    # 2. tools/list
    proc.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}) + "\n")
    proc.stdin.flush()
    tools = recv(proc)
    names = [t["name"] for t in tools["result"]["tools"]]
    print(f"[smoke] tools/list: {len(names)} tools advertised")
    assert "kb_write_page" in names and "kb_search" in names and "kb_get_page" in names, names

    # 3. kb_write_page
    wr = {
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "kb_write_page",
            "arguments": {"path": "smoke/hello.md", "content": "A zebra walks through the savanna."},
        },
    }
    proc.stdin.write(json.dumps(wr) + "\n")
    proc.stdin.flush()
    wresp = recv(proc)
    content = wresp["result"]["structuredContent"]
    print(f"[smoke] kb_write_page ok: {content['path']} blocks={content['block_count']}")
    assert not wresp["result"]["isError"], wresp

    # 4. kb_get_page
    gp = {
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {"name": "kb_get_page", "arguments": {"path": "smoke/hello.md"}},
    }
    proc.stdin.write(json.dumps(gp) + "\n")
    proc.stdin.flush()
    gresp = recv(proc)
    gcontent = gresp["result"]["structuredContent"]
    print(f"[smoke] kb_get_page ok: title={gcontent['title']!r} content={gcontent['content']!r}")
    assert gcontent["content"] == "A zebra walks through the savanna."

    # 5. kb_get_page for a missing note (must be graceful, not crash)
    gp_missing = {
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {"name": "kb_get_page", "arguments": {"path": "does/not/exist.md"}},
    }
    proc.stdin.write(json.dumps(gp_missing) + "\n")
    proc.stdin.flush()
    mresp = recv(proc)
    print(f"[smoke] kb_get_page missing -> isError={mresp['result']['isError']} err={mresp['result'].get('structuredContent', {}).get('error')}")
    assert mresp["result"]["isError"] is True

    # 6. kb_search for the written content — acceptance: content search works
    sr = {
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {"name": "kb_search", "arguments": {"query": "zebra", "limit": 5}},
    }
    proc.stdin.write(json.dumps(sr) + "\n")
    proc.stdin.flush()
    sresp = recv(proc)
    sc = sresp["result"]["structuredContent"]
    print(f"[smoke] kb_search zebra -> total={sc['total']} hits={len(sc['results'])}")
    assert sc["total"] >= 1
    hit = sc["results"][0]
    print(f"[smoke]   hit content={hit['content']!r} page={hit['page_path']}")
    assert hit["content"] == "A zebra walks through the savanna."

    proc.stdin.close()
    proc.wait(timeout=10)
    print("\n[smoke] ALL PASS")


if __name__ == "__main__":
    main()

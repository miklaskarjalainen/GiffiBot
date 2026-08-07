#!/usr/bin/env python3
"""Benchmark GiffiBot's move ordering / alpha-beta pruning over UCI positions.

Drives the engine over a set of FEN positions with a fixed-depth search and
reports node counts, timing, nps and effective branching factor per position.
All results go to stdout so runs can be piped to files and diffed between
engine builds; progress/diagnostics go to stderr.

With --compare a second engine binary is run over the same positions and a
comparison summary (average time/nodes/nps per engine, per-position time
ratio, worst positions) is printed so you can decide which is faster.

Usage:
    python3 testing/benchmark.py --engine target/release/giffibot --depth 7
    python3 testing/benchmark.py --nodes-only > run_a.txt
    python3 testing/benchmark.py --compare target/release/giffibot_old --limit 20
    python3 testing/benchmark.py --repeats 3 --limit 10
"""

import argparse
import math
import os
import re
import selectors
import statistics
import subprocess
import sys
import time

INFO_RE = re.compile(
    r"info depth (?P<depth>\d+) "
    r"score (?P<score>cp -?\d+|mate -?\d+) "
    r"currmove (?P<currmove>\S+) "
    r"nodes (?P<nodes>\d+) "
    r"time (?P<time>[\d.]+) "
    r"nps (?P<nps>\d+) "
    r"pv (?P<pv>.*)"
)


def load_positions(path):
    positions = []
    with open(path, "r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#") or line.startswith("//"):
                continue
            fen = line.rstrip(";").strip()
            if fen:
                positions.append(fen)
    return positions


class EngineLineReader:
    def __init__(self, proc, timeout):
        self.proc = proc
        self.timeout = timeout
        self.fd = proc.stdout.fileno()
        os.set_blocking(self.fd, False)
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.fd, selectors.EVENT_READ)
        self.buf = b""

    def read_line(self, timeout=None):
        deadline = time.monotonic() + (timeout if timeout is not None else self.timeout)
        while True:
            nl = self.buf.find(b"\n")
            if nl != -1:
                line = self.buf[:nl]
                self.buf = self.buf[nl + 1 :]
                return line.decode("utf-8", "replace").rstrip("\r")
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return None
            ready = self.selector.select(remaining)
            if not ready:
                return None
            chunk = os.read(self.fd, 65536)
            if not chunk:
                return None
            self.buf += chunk

    def drain(self, pred, timeout=None):
        lines = []
        while True:
            line = self.read_line(timeout)
            if line is None:
                return lines, None
            lines.append(line)
            if pred(line):
                return lines, line


def parse_info(line):
    m = INFO_RE.match(line)
    if not m:
        return None
    score_str = m.group("score")
    if score_str.startswith("cp "):
        score = int(score_str[3:])
        is_mate = False
    else:
        score = int(score_str[5:])
        is_mate = True
    return {
        "depth": int(m.group("depth")),
        "score": score,
        "is_mate": is_mate,
        "currmove": m.group("currmove"),
        "nodes": int(m.group("nodes")),
        "time": float(m.group("time")) / 100.0,
        "nps": int(m.group("nps")),
        "pv": m.group("pv").strip(),
    }


def send_position(reader, fen):
    """Send a position and wait for the engine to confirm it via isready.
    Returns (True, None) or (False, error). No search is started yet, so a bad
    FEN cannot leak a stray search thread into the following position."""
    proc = reader.proc
    proc.stdin.write(f"position fen {fen}\nisready\n".encode())
    proc.stdin.flush()

    error = None
    while True:
        line = reader.read_line()
        if line is None:
            return False, "timeout"
        if "FEN PARSE ERROR" in line:
            error = "fen-parse-error"
            continue
        if line.strip() == "readyok":
            break
    if error:
        return False, error
    return True, None


def run_go(reader, depth):
    """Start a fixed-depth search and collect info lines until bestmove.
    Returns (True, info_by_depth) or (False, error)."""
    proc = reader.proc
    proc.stdin.write(f"go depth {depth}\n".encode())
    proc.stdin.flush()

    info_by_depth = {}
    while True:
        line = reader.read_line()
        if line is None:
            return False, "timeout"
        if line.startswith("info "):
            parsed = parse_info(line)
            if parsed is not None:
                info_by_depth[parsed["depth"]] = parsed
            continue
        if line.startswith("bestmove"):
            break
    if not info_by_depth:
        return False, "no-info"
    return True, info_by_depth


def compute_metrics(info_by_depth):
    depths = sorted(info_by_depth)
    last = info_by_depth[depths[-1]]
    nodes = last["nodes"]
    time_s = last["time"]
    nps = nodes / time_s if time_s > 0 else 0.0
    ebf = None
    if len(depths) >= 2:
        prev = info_by_depth[depths[-2]]
        exact_cur = last["nodes"] - prev["nodes"]
        if prev["nodes"] > 0:
            ebf = exact_cur / prev["nodes"]
    return {
        "depth": depths[-1],
        "nodes": nodes,
        "time_s": time_s,
        "nps": nps,
        "ebf": ebf,
        "best": last["pv"].split()[0] if last["pv"] else "",
        "score": last["score"],
        "is_mate": last["is_mate"],
        "info": info_by_depth,
    }


def per_depth_rows(metrics):
    depths = sorted(metrics["info"])
    rows = []
    prev_nodes = 0
    prev_time = 0.0
    prev_exact = None
    for d in depths:
        entry = metrics["info"][d]
        exact = entry["nodes"] - prev_nodes
        time_delta = entry["time"] - prev_time
        nps = exact / time_delta if time_delta > 0 else 0.0
        ebf = (exact / prev_exact) if prev_exact else None
        rows.append((d, entry["nodes"], exact, time_delta, nps, ebf))
        prev_nodes = entry["nodes"]
        prev_time = entry["time"]
        prev_exact = exact
    return rows


class EngineSession:
    """One engine subprocess, driven over positions via UCI."""

    def __init__(self, path, timeout):
        self.path = path
        self.timeout = timeout
        self.proc = subprocess.Popen(
            [path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        self.reader = EngineLineReader(self.proc, timeout)

    def handshake(self):
        self.proc.stdin.write(b"uci\nisready\n")
        self.proc.stdin.flush()
        _, ready = self.reader.drain(lambda line: line.strip() == "readyok")
        return ready is not None

    def search(self, fen, depth, repeats=1):
        """Search one position. Returns (True, info_by_depth) or (False, error).
        With repeats > 1 the position is searched N times and the median
        duration per depth is reported (node counts are deterministic)."""
        ok, err = send_position(self.reader, fen)
        if not ok:
            return False, err
        runs = []
        for _ in range(repeats):
            ok, payload = run_go(self.reader, depth)
            if not ok:
                return False, payload
            runs.append(payload)
        if repeats == 1:
            return True, runs[0]
        merged = {}
        for d in runs[0]:
            entry = dict(runs[0][d])
            entry["time"] = statistics.median(r[d]["time"] for r in runs)
            merged[d] = entry
        return True, merged

    def close(self):
        try:
            self.proc.stdin.write(b"quit\n")
            self.proc.stdin.flush()
        except (BrokenPipeError, ValueError):
            pass
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()


def print_engine_table(engine, depth, positions_count, results, per_depth, totals, ebfs):
    print(f"# giffibot bench engine={engine}")
    print(f"# positions={positions_count} depth={depth}")
    print("# idx\tbest\tdepth\tnodes\ttime_s\tnps\tebf\tfen")
    for idx, (ok, payload, fen) in enumerate(results, 1):
        if ok:
            ebf = f"{payload['ebf']:.2f}" if payload["ebf"] is not None else "-"
            print(
                f"{idx}\t{payload['best']}\t{payload['depth']}\t{payload['nodes']}\t"
                f"{payload['time_s']:.4f}\t{payload['nps']:.0f}\t{ebf}\t{fen}"
            )
        else:
            print(f"{idx}\t-\t-\t-\t-\t-\t{payload}\t{fen}")

    if per_depth:
        print("# per-depth: idx depth nodes exact_nodes time_s nps ebf")
        for idx, (ok, payload, _) in enumerate(results, 1):
            if not ok:
                continue
            for d, nodes, exact, t, nps, ebf in per_depth_rows(payload):
                ebf_s = f"{ebf:.2f}" if ebf is not None else "-"
                print(f"{idx}\t{d}\t{nodes}\t{exact}\t{t:.4f}\t{nps:.0f}\t{ebf_s}")

    ok_count = sum(1 for ok, _, _ in results if ok)
    fail_count = len(results) - ok_count
    avg_nps = totals["nodes"] / totals["time"] if totals["time"] > 0 else 0.0
    geo_ebf = math.exp(sum(math.log(e) for e in ebfs) / len(ebfs)) if ebfs else 0.0
    print("# summary")
    print(f"# positions_ok={ok_count} failed={fail_count}")
    print(f"# total_nodes={totals['nodes']} total_time_s={totals['time']:.4f} avg_nps={avg_nps:.0f}")
    print(f"# geomean_ebf={geo_ebf:.3f}")


def print_comparison(results_a, results_b, engine_a, engine_b):
    pairs = []
    skip_a = 0
    skip_b = 0
    for (ok_a, metrics_a, _), (ok_b, metrics_b, _) in zip(results_a, results_b):
        if ok_a and ok_b:
            pairs.append((metrics_a, metrics_b))
        else:
            skip_a += not ok_a
            skip_b += not ok_b

    print(f"# comparison engineA={engine_a} engineB={engine_b}")
    print(f"# compared={len(pairs)} skipA={skip_a} skipB={skip_b}")
    if not pairs:
        print("# no comparable positions")
        return

    for label, pick in (("engineA", 0), ("engineB", 1)):
        metrics_list = [p[pick] for p in pairs]
        avg_time = statistics.mean(m["time_s"] for m in metrics_list)
        avg_nodes = statistics.mean(m["nodes"] for m in metrics_list)
        avg_nps = statistics.mean(m["nps"] for m in metrics_list)
        total_time = sum(m["time_s"] for m in metrics_list)
        total_nodes = sum(m["nodes"] for m in metrics_list)
        print(f"# avg_per_position {label}: time={avg_time:.4f}s nodes={avg_nodes:.0f} nps={avg_nps:.0f}")
        print(f"# totals {label}: time={total_time:.4f}s nodes={total_nodes}")

    ratios = [ma["time_s"] / mb["time_s"] for ma, mb in pairs if mb["time_s"] > 0]
    geo = math.exp(sum(math.log(r) for r in ratios) / len(ratios)) if ratios else 0.0
    a_fast = sum(1 for ma, mb in pairs if ma["time_s"] < mb["time_s"])
    b_fast = sum(1 for ma, mb in pairs if mb["time_s"] < ma["time_s"])
    tie = len(pairs) - a_fast - b_fast
    print(f"# time_ratio_geomean (A/B)={geo:.3f}  faster: A={a_fast} B={b_fast} tie={tie}")

    for label, results in (("engineA", results_a), ("engineB", results_b)):
        oks = [(m, fen) for ok, m, fen in results if ok]
        if not oks:
            continue
        slowest = max(oks, key=lambda pair: pair[0]["time_s"])
        m, fen = slowest
        print(
            f"# worst_by_time {label}: idx={oks.index(slowest) + 1} time={m['time_s']:.4f}s "
            f"nodes={m['nodes']} nps={m['nps']:.0f} fen={fen}"
        )
        biggest = max(oks, key=lambda pair: pair[0]["nodes"])
        m, fen = biggest
        print(
            f"# worst_by_nodes {label}: idx={oks.index(biggest) + 1} nodes={m['nodes']} "
            f"time={m['time_s']:.4f}s fen={fen}"
        )


def main():
    ap = argparse.ArgumentParser(
        description="Benchmark GiffiBot move ordering / alpha-beta pruning over UCI positions."
    )
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    ap.add_argument(
        "--engine", default=os.path.join(root, "target", "release", "giffibot")
    )
    ap.add_argument(
        "--compare", default=None, help="second engine binary to compare against"
    )
    ap.add_argument(
        "--positions", default=os.path.join(root, "testing", "positions.txt")
    )
    ap.add_argument("--depth", type=int, default=7)
    ap.add_argument(
        "--limit", type=int, default=None, help="only benchmark the first N positions"
    )
    ap.add_argument(
        "--repeats",
        type=int,
        default=1,
        help="run each engine N times per position, report median time (runtime scales by N)",
    )
    ap.add_argument(
        "--timeout", type=float, default=300.0, help="per-position timeout in seconds"
    )
    ap.add_argument(
        "--nodes-only",
        action="store_true",
        help="print only 'nodes<TAB>fen' per position, for diffing builds",
    )
    ap.add_argument(
        "--per-depth",
        action="store_true",
        help="also print per-depth nodes and effective branching factor",
    )
    args = ap.parse_args()
    os.system("cargo build --release")
    engine = os.path.abspath(args.engine)
    if not os.path.isfile(engine) or not os.access(engine, os.X_OK):
        print(f"error: engine not found or not executable: {engine}", file=sys.stderr)
        sys.exit(1)

    compare = os.path.abspath(args.compare) if args.compare else None
    if compare is not None:
        if not os.path.isfile(compare) or not os.access(compare, os.X_OK):
            print(f"error: compare engine not found or not executable: {compare}", file=sys.stderr)
            sys.exit(1)
    if args.repeats < 1:
        print("error: --repeats must be >= 1", file=sys.stderr)
        sys.exit(1)

    positions = load_positions(args.positions)
    if not positions:
        print(f"error: no positions found in {args.positions}", file=sys.stderr)
        sys.exit(1)
    if args.limit:
        positions = positions[: args.limit]
    if args.depth < 1:
        print("error: --depth must be >= 1", file=sys.stderr)
        sys.exit(1)

    sessions = [EngineSession(engine, args.timeout)]
    if compare is not None:
        sessions.append(EngineSession(compare, args.timeout))
    labels = [os.path.basename(s.path) for s in sessions]

    for s in sessions:
        if not s.handshake():
            print(f"error: engine did not respond to isready: {s.path}", file=sys.stderr)
            for x in sessions:
                x.close()
            sys.exit(1)

    results = [[] for _ in sessions]
    totals = [{"nodes": 0, "time": 0.0} for _ in sessions]
    ebfs = [[] for _ in sessions]

    try:
        for i, fen in enumerate(positions, 1):
            t0 = time.monotonic()
            statuses = []
            for j, session in enumerate(sessions):
                ok, payload = session.search(fen, args.depth, args.repeats)
                if ok:
                    metrics = compute_metrics(payload)
                    results[j].append((True, metrics, fen))
                    totals[j]["nodes"] += metrics["nodes"]
                    totals[j]["time"] += metrics["time_s"]
                    if metrics["ebf"] is not None:
                        ebfs[j].append(metrics["ebf"])
                    statuses.append(f"{labels[j]}={metrics['time_s']:.1f}s")
                else:
                    results[j].append((False, payload, fen))
                    statuses.append(f"{labels[j]}={payload}")
            elapsed = time.monotonic() - t0
            print(
                f"[{i}/{len(positions)}] {elapsed:.1f}s {' '.join(statuses)} {fen[:60]}",
                file=sys.stderr,
            )

        if args.nodes_only:
            for j, session in enumerate(sessions):
                print(f"# giffibot bench nodes depth={args.depth} engine={session.path}")
                for ok, payload, fen in results[j]:
                    if ok:
                        print(f"{payload['nodes']}\t{fen}")
                    else:
                        print(f"ERR\t{fen}")
            return

        for j, session in enumerate(sessions):
            print_engine_table(
                session.path, args.depth, len(positions), results[j], args.per_depth,
                totals[j], ebfs[j],
            )
            if j < len(sessions) - 1:
                print()

        if len(sessions) == 2:
            print_comparison(results[0], results[1], sessions[0].path, sessions[1].path)
    finally:
        for session in sessions:
            session.close()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)

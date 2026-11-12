#!/usr/bin/env python3
"""Run the Julia stages of the MSL sweep over a pool of long-lived workers.

    python3 -I run_sweep.py --work WORK --refs REFS --models models.txt \
        [--jobs 4] [--dispatch-minutes 18] [--hard-minutes 29]

For each model whose Base Modelica was emitted (WORK/emit/*.tsv, written by
emit.sh) and that has no WORK/rows/<model>.tsv yet, one worker.jl process runs
parse -> convert -> simplify -> init -> solve -> compare and writes the row.
The scalarized output (base-modelica-scalar) is used when it exists, the
array-preserving one otherwise.

Timeouts are enforced here: a stage longer than --stage-timeout (300 s) or a
model longer than --model-timeout (600 s) gets its worker killed, a Timeout row
written, and a fresh worker started. A worker that dies is recorded as
WorkerCrash. New models are dispatched until --dispatch-minutes; at
--hard-minutes anything still running is killed without a row, so a re-run
picks it up. Re-running skips finished models: the sweep is resumable.
"""
import argparse
import os
import queue
import signal
import subprocess
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
COLS = [
    "model", "bmo", "stage_reached", "failed_stage", "status", "error_type", "error",
    "n_unknowns_raw", "n_unknowns", "n_observed", "solver", "retcode", "tspan",
    "n_signals", "n_found", "max_rel_err", "max_rel_err_strict", "worst_signal",
    "missing_signals", "compare_note",
    "t_parse", "t_convert", "t_simplify", "t_init", "t_solve", "t_compare",
]
STAGES = ["parse", "convert", "simplify", "init", "solve", "compare"]


def emitted(work, model):
    """Return the .bmo to run: scalar if it was emitted, else array, else None."""
    fname = model.replace(".", "_") + ".bmo"
    for target in ("base-modelica-scalar", "base-modelica"):
        row = os.path.join(work, "emit", f"{model}.{target}.tsv")
        if os.path.exists(row):
            with open(row) as fh:
                fields = fh.read().rstrip("\n").split("\t")
            if len(fields) > 2 and fields[2] == "ok":
                return os.path.join(work, "bmo", target, fname)
    return None


def rss_gb(pid):
    try:
        with open(f"/proc/{pid}/status") as fh:
            for line in fh:
                if line.startswith("VmRSS:"):
                    return int(line.split()[1]) / 1e6
    except OSError:
        pass
    return 0.0


class Worker:
    def __init__(self, args, slot):
        self.args, self.slot, self.n = args, slot, 0
        self.proc = None

    def start(self):
        self.n += 1
        log = open(os.path.join(self.args.work, "wlog", f"worker{self.slot}-{self.n}.log"), "w")
        cmd = [self.args.julia, "--startup-file=no", f"--project={self.args.project}",
               f"--heap-size-hint={self.args.heap}", os.path.join(HERE, "worker.jl"), self.args.warmup]
        self.proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=log, text=True, bufsize=1, start_new_session=True)
        self.lines = queue.Queue()
        self.jobs_done = 0
        threading.Thread(target=self._read, args=(self.proc, self.lines), daemon=True).start()

    @staticmethod
    def _read(proc, q):
        for line in proc.stdout:
            q.put(line.rstrip("\n"))
        q.put(None)

    def kill(self):
        if self.proc and self.proc.poll() is None:
            try:
                os.killpg(self.proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            self.proc.wait()
        self.proc = None

    def wait_ready(self, timeout):
        end = time.time() + timeout
        while time.time() < end:
            try:
                line = self.lines.get(timeout=5)
            except queue.Empty:
                continue
            if line is None:
                return False
            if line == "@@READY":
                return True
        return False


def write_row(path, fields):
    row = {c: "" for c in COLS}
    row.update(fields)
    with open(path, "w") as fh:
        fh.write("\t".join(str(row[c]).replace("\t", " ") for c in COLS) + "\n")


def run_slot(args, slot, todo, t_start, stats):
    w = Worker(args, slot)
    while True:
        now = time.time()
        if now - t_start > args.dispatch_minutes * 60:
            break
        try:
            model, bmo = todo.get_nowait()
        except queue.Empty:
            break
        if w.proc is None:
            w.start()
            if not w.wait_ready(900):
                print(f"[slot {slot}] worker failed to start; see wlog", flush=True)
                w.kill()
                todo.put((model, bmo))
                break
        name = model
        rowf = os.path.join(args.work, "rows", f"{name}.tsv")
        logf = os.path.join(args.work, "jlog", f"{name}.log")
        refdir = os.path.join(args.refs, *model.split("."))
        if not os.path.isdir(refdir):
            refdir = "-"
        w.proc.stdin.write(f"{model}\t{bmo}\t{refdir}\t{rowf}\t{logf}\n")
        w.proc.stdin.flush()
        t_model = time.time()
        stage, t_stage = "parse", t_model
        times, done, outcome = {}, False, None
        while not done:
            hard_left = t_start + args.hard_minutes * 60 - time.time()
            left = min(args.stage_timeout - (time.time() - t_stage),
                       args.model_timeout - (time.time() - t_model), hard_left)
            if left <= 0:
                outcome = "hard" if hard_left <= 0 else "timeout"
                break
            try:
                line = w.lines.get(timeout=min(left, 5))
            except queue.Empty:
                continue
            if line is None:
                outcome = "crash"
                break
            if line.startswith("@@STAGE "):
                new = line.split()[1]
                times[stage] = round(time.time() - t_stage, 2)
                stage, t_stage = new, time.time()
            elif line == "@@DONE":
                done = True
        elapsed = time.time() - t_model
        if done:
            stats["done"] += 1
            w.jobs_done += 1
            if w.jobs_done >= args.recycle or rss_gb(w.proc.pid) > args.max_rss:
                w.kill()
            print(f"[slot {slot}] {model}: {elapsed:.0f}s", flush=True)
            continue
        w.kill()
        if outcome == "hard":
            print(f"[slot {slot}] {model}: cut at batch end, will rerun", flush=True)
            break
        reached = STAGES[STAGES.index(stage) - 1] if stage in STAGES and STAGES.index(stage) > 0 else "emit"
        fields = {"model": model, "bmo": os.path.basename(bmo), "stage_reached": reached,
                  "failed_stage": stage, "status": "fail"}
        for s, t in times.items():
            if s in STAGES:
                fields["t_" + s] = t
        fields["t_" + stage] = round(time.time() - t_stage, 2)
        if outcome == "timeout":
            which = "stage" if time.time() - t_stage >= args.stage_timeout - 1 else "model"
            fields["error_type"] = "Timeout"
            fields["error"] = f"{which} timeout in {stage} after {elapsed:.0f}s"
        else:
            fields["error_type"] = "WorkerCrash"
            tail = ""
            try:
                with open(os.path.join(args.work, "wlog", f"worker{slot}-{w.n}.log"), errors="replace") as fh:
                    tail = " | ".join(l.strip() for l in fh.read().splitlines()[-3:])
            except OSError:
                pass
            fields["error"] = f"worker died in {stage}: {tail}"[:400]
        write_row(rowf, fields)
        stats["done"] += 1
        print(f"[slot {slot}] {model}: {fields['error_type']} in {stage}", flush=True)
    w.kill()


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--work", required=True)
    p.add_argument("--refs", required=True, help="MAP-LIB_ReferenceResults checkout")
    p.add_argument("--models", required=True)
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--project", default=os.path.join(HERE, "julia"))
    p.add_argument("--julia", default="julia")
    p.add_argument("--warmup", default=os.path.join(HERE, "warmup.bmo"))
    p.add_argument("--heap", default="5G")
    p.add_argument("--stage-timeout", type=float, default=300)
    p.add_argument("--model-timeout", type=float, default=600)
    p.add_argument("--dispatch-minutes", type=float, default=18)
    p.add_argument("--hard-minutes", type=float, default=29)
    p.add_argument("--recycle", type=int, default=20)
    p.add_argument("--max-rss", type=float, default=7.0)
    p.add_argument("--only", default="", help="substring filter on model names")
    args = p.parse_args()
    for d in ("rows", "jlog", "wlog"):
        os.makedirs(os.path.join(args.work, d), exist_ok=True)
    todo = queue.Queue()
    n_skip = n_noemit = 0
    with open(args.models) as fh:
        models = [l.strip() for l in fh if l.strip()]
    for m in models:
        if args.only and args.only not in m:
            continue
        if os.path.exists(os.path.join(args.work, "rows", f"{m}.tsv")):
            n_skip += 1
            continue
        bmo = emitted(args.work, m)
        if bmo is None:
            n_noemit += 1
            continue
        todo.put((m, bmo))
    print(f"queued {todo.qsize()}, already done {n_skip}, not emitted {n_noemit}", flush=True)
    stats = {"done": 0}
    t_start = time.time()
    threads = [threading.Thread(target=run_slot, args=(args, i, todo, t_start, stats))
               for i in range(args.jobs)]
    for i, t in enumerate(threads):
        t.start()
        time.sleep(2)
    for t in threads:
        t.join()
    print(f"batch finished: {stats['done']} rows in {time.time() - t_start:.0f}s, "
          f"{todo.qsize()} left in queue", flush=True)


if __name__ == "__main__":
    main()

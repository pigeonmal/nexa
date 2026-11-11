#!/usr/bin/env python3
"""Summarize a saved Release Perfetto trace without capturing screenshots.

Requires the optional developer-only `perfetto` Python package. Process filters
are applied to all app metrics. No package is installed or launched by this tool.
"""
import argparse
import hashlib
import json
from pathlib import Path

QUERIES = {
    "bounds": "select * from trace_bounds",
    "processes": "select pid,name from process where name=$PACKAGE",
    "issues": "select severity,name,value from stats where severity in ('error','data_loss') and value>0",
    "threadCpu": """select t.name,t.is_main_thread,round(sum(s.dur)/1e6,3) cpu_ms,
        count(*) scheduler_runs from sched s join thread t using(utid)
        join process p using(upid) where p.name=$PACKAGE and s.dur>0
        group by t.utid order by cpu_ms desc""",
    "sampleCount": """select count(*) samples from perf_sample ps join thread t using(utid)
        join process p using(upid) where p.name=$PACKAGE""",
    "sampleLeaves": """select coalesce(f.name,m.name,'unknown') frame,t.name thread_name,
        count(*) samples from perf_sample ps join thread t using(utid)
        join process p using(upid) left join stack_profile_callsite c on c.id=ps.callsite_id
        left join stack_profile_frame f on f.id=c.frame_id
        left join stack_profile_mapping m on m.id=f.mapping where p.name=$PACKAGE
        group by frame,t.name order by samples desc limit 25""",
    "mainSlices": """select sl.name,count(*) count,round(sum(sl.dur)/1e6,3) ms from slice sl
        join thread_track tt on sl.track_id=tt.id join thread t using(utid)
        join process p using(upid) where p.name=$PACKAGE and t.is_main_thread=1 and sl.dur>0
        group by sl.name order by ms desc limit 30""",
    "layoutSlices": """select sl.name,count(*) count,round(sum(sl.dur)/1e6,3) ms from slice sl
        join thread_track tt on sl.track_id=tt.id join thread t using(utid)
        join process p using(upid) where p.name=$PACKAGE and
        (lower(sl.name) like '%measure%' or lower(sl.name) like '%layout%' or lower(sl.name) like '%compos%')
        group by sl.name order by ms desc limit 30""",
    "frames": """select jank_type,count(*) count,round(avg(a.dur)/1e6,3) avg_ms,
        round(max(a.dur)/1e6,3) max_ms from actual_frame_timeline_slice a
        join process p using(upid) where p.name=$PACKAGE and a.dur>0 group by jank_type""",
    "memoryCounters": """select c.name,round(min(value),0) min_value,round(max(value),0) max_value,
        count(*) samples from counter join process_counter_track c on counter.track_id=c.id
        join process p using(upid) where p.name=$PACKAGE group by c.name""",
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path)
    parser.add_argument("--package", default="dev.nexa.performanceprofile")
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    try:
        from perfetto.trace_processor import TraceProcessor
    except ImportError as error:
        raise SystemExit("Install the optional perfetto Python package in a profiling venv") from error
    # SQL literals are escaped independently of shell arguments.
    package = "'" + args.package.replace("'", "''") + "'"
    with TraceProcessor(trace=str(args.trace)) as processor:
        result = {name: [vars(row) for row in processor.query(sql.replace("$PACKAGE", package))]
                  for name, sql in QUERIES.items()}
    if not result["processes"]:
        raise SystemExit("Target process is missing; zero CPU is not valid idle evidence")
    result["traceSha256"] = hashlib.sha256(args.trace.read_bytes()).hexdigest()
    result["package"] = args.package
    result["scope"] = "Release app trace; no hardware frame-rate or battery guarantee"
    output = json.dumps(result, indent=2) + "\n"
    if args.out:
        args.out.write_text(output)
    else:
        print(output, end="")
    if result["issues"]:
        raise SystemExit("Trace contains error/data-loss diagnostics; inspect before accepting metrics")


if __name__ == "__main__":
    main()

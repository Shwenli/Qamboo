#!/usr/bin/env python3
"""
Extract ORQ query-experiment results from log files into CSV.

Adapted from the parsing part of plot_query_experiments.py (ORQ's plotting
script). ORQ writes one log file per query run; the AE scripts run it from
the checkout at nsdi27-ae/baselines/orq (installed by
nsdi27-ae/setup/setup_orq.sh), where the results land under
baselines/orq/experiments/results/.

Typical log content (one file per query run):

    Q4 SF 1
    L size: 5997482
    O size: 1500000
    [=SW]            Start
    [ SW]          Filters 0.5396   sec
    [ SW]             Join 94.72    sec
    [ SW]         Distinct 36.4     sec
    [ SW]            Count 64.82    sec
    [=SW]          Overall 206      sec

Recognized lines (both "[ SW]" and "[=SW]" bracket styles):
    [ SW] <stage> <time> sec      -> per-stage time; stage names vary per
                                     query and become CSV columns
    [=SW] Overall <time> sec      -> overall execution time
    <query> SF <n> / SF; <n>      -> scale factor
    ... threads; <n>              -> thread count (optional)
    [=COMM] <what> <value> ...    -> communication counters (optional)
    [=PREPROC] ... <time> sec     -> preprocessing time (2PC only, optional)

Files without an "Overall" line are skipped as incomplete runs.

Usage:
  # one CSV row per run file
  python3 extract_orq_log.py -i ../baselines/orq/experiments/results/tpch

  # median per query (multiple runs of the same query)
  python3 extract_orq_log.py -i results/ --median -o fig7_orq.csv

  # 2PC logs: subtract the preprocessing time from the overall time
  python3 extract_orq_log.py -i results/ --twopc
"""

import argparse
import csv
import glob
import os
import re
import statistics
import sys
from pathlib import Path

# "[ SW]  Filters 0.5396  sec" / "[=SW]  Overall 206  sec"
STAGE_RE = re.compile(r'\[\s*=?\s*SW\]\s+(\S[^0-9]*?)\s+([\d.]+)\s*sec\b')
# "Q4 SF 1" or "Q1 SF; 1 threads; 16"
QUERY_SF_RE = re.compile(r'^([A-Za-z]\w*)\s+SF[;]?\s+(\d+)')
SF_RE = re.compile(r'\bSF[;]?\s+(\d+)')
THREADS_RE = re.compile(r'\bthreads[;]?\s+(\d+)')
# "[=COMM] table.sort 1024 ..."
COMM_RE = re.compile(r'\[=COMM\]\s+(\S+)\s+([\d.]+)')
# "[=PREPROC] dm-dummyperm 0.5 sec"
PREPROC_RE = re.compile(r'\[=PREPROC\].*?([\d.]+)\s*sec\b')

FIXED_FIELDS = ['query', 'sf', 'threads', 'overall_time(s)']
TAIL_FIELDS = ['preproc(s)', 'file']


def natural_key(name):
    """Sort query names naturally: q2 before q10."""
    return [int(t) if t.isdigit() else t.lower() for t in re.split(r'(\d+)', name)]


def parse_orq_file(path):
    """Parse one ORQ per-query log file.

    Returns (record, None) or (None, reason) for incomplete runs. record:
    overall, stages {name: time}, comm {what: value}, sf, threads, preproc.
    """
    rec = {'overall': None, 'stages': {}, 'comm': {}, 'query_hint': None,
           'sf': None, 'threads': None, 'preproc': None}

    with open(path, 'r', encoding='utf-8') as f:
        for line in f:
            stage_match = STAGE_RE.search(line)
            if stage_match:
                name = stage_match.group(1).strip()
                value = float(stage_match.group(2))
                if name == 'Overall':
                    rec['overall'] = value
                else:
                    rec['stages'][name] = value
                continue

            if 'Overall' in line:
                # Fallback for "Overall time: 12.345" style lines.
                spl = line.split()
                if len(spl) > 2:
                    try:
                        rec['overall'] = float(spl[2])
                    except ValueError:
                        pass

            query_sf_match = QUERY_SF_RE.search(line)
            if query_sf_match:
                rec['query_hint'] = query_sf_match.group(1)
                rec['sf'] = int(query_sf_match.group(2))
            else:
                sf_match = SF_RE.search(line)
                if sf_match:
                    rec['sf'] = int(sf_match.group(1))
            threads_match = THREADS_RE.search(line)
            if threads_match:
                rec['threads'] = int(threads_match.group(1))

            comm_match = COMM_RE.search(line)
            if comm_match:
                rec['comm'][comm_match.group(1)] = float(comm_match.group(2))

            preproc_match = PREPROC_RE.search(line)
            if preproc_match:
                rec['preproc'] = float(preproc_match.group(1))

    if rec['overall'] is None:
        return None, 'no Overall line'
    return rec, None


def collect_runs(input_path, twopc):
    """Parse all *.txt logs below a directory (or a single file).

    Returns (runs, stage_names, comm_names) with names in first-seen order.
    """
    if os.path.isdir(input_path):
        files = sorted(glob.glob(os.path.join(input_path, '**', '*.txt'),
                                 recursive=True))
    else:
        files = [input_path]

    runs = []
    stage_names = []
    comm_names = []
    for file_name in files:
        query_name = Path(file_name).stem
        if query_name == 'meta':
            continue  # metadata file, not a query
        rec, reason = parse_orq_file(file_name)
        if rec is None:
            print(f'Note: skipping incomplete log {file_name} ({reason})',
                  file=sys.stderr)
            continue

        overall = rec['overall']
        if twopc:
            if rec['preproc'] is None:
                print(f'WARNING: {query_name} has no preprocessing time.',
                      file=sys.stderr)
            else:
                overall -= rec['preproc']

        # Prefer the query id from the "Qx SF n" header line over the filename.
        query_name = rec['query_hint'] or query_name

        for name in rec['stages']:
            if name not in stage_names:
                stage_names.append(name)
        for name in rec['comm']:
            if name not in comm_names:
                comm_names.append(name)

        runs.append({
            'query': query_name,
            'sf': rec['sf'] if rec['sf'] is not None else 'N/A',
            'threads': rec['threads'] if rec['threads'] is not None else 'N/A',
            'overall_time(s)': overall,
            'stages': rec['stages'],
            'comm': rec['comm'],
            'preproc(s)': rec['preproc'] if rec['preproc'] is not None else 'N/A',
            'file': file_name,
        })
    return runs, stage_names, comm_names


def flatten(run, stage_names, comm_names):
    """Turn one run into a flat CSV row (missing stages/comm become N/A)."""
    row = {k: run[k] for k in FIXED_FIELDS + TAIL_FIELDS}
    for name in stage_names:
        row[f'{name}(s)'] = run['stages'].get(name, 'N/A')
    for name in comm_names:
        row[f'{name}_comm'] = run['comm'].get(name, 'N/A')
    return row


def median_per_query(runs, stage_names, comm_names):
    """Collapse repeated runs of the same query to their per-field median."""
    numeric_fields = (['overall_time(s)'] + [f'{n}(s)' for n in stage_names]
                      + [f'{n}_comm' for n in comm_names] + ['preproc(s)'])

    by_query = {}
    for run in runs:
        by_query.setdefault(run['query'], []).append(run)

    out = []
    for query in sorted(by_query, key=natural_key):
        group = by_query[query]
        flats = [flatten(g, stage_names, comm_names) for g in group]
        row = {'query': query, 'runs': len(group)}
        for field in ['sf', 'threads'] + numeric_fields:
            values = [f[field] for f in flats if f[field] != 'N/A']
            value = statistics.median(values) if values else 'N/A'
            # sf/threads are integers by nature; keep them displayed as such.
            if field in ('sf', 'threads') and value != 'N/A':
                value = int(value)
            row[field] = value
        # Fewer than 3 runs, as flagged in the original script.
        row['flag'] = '###' if len(group) < 3 else ''
        out.append(row)
    return out


def main():
    parser = argparse.ArgumentParser(
        description='Extract ORQ query-experiment results into CSV.',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument('-i', '--input', required=True,
                        help='ORQ results directory (searched recursively for '
                             '*.txt) or a single log file')
    parser.add_argument('-o', '--output', default=None,
                        help='output CSV file (default: <input name>.csv)')
    parser.add_argument('--median', action='store_true',
                        help='collapse repeated runs of the same query to the '
                             'per-query median instead of one row per run')
    parser.add_argument('--twopc', action='store_true',
                        help='2PC logs: subtract the [=PREPROC] preprocessing '
                             'time from the overall time')
    args = parser.parse_args()

    if args.output is None:
        base = os.path.basename(os.path.normpath(args.input))
        args.output = os.path.splitext(base)[0] + '.csv'
    elif not args.output.endswith('.csv'):
        args.output += '.csv'

    if not os.path.exists(args.input):
        print(f'Error: input not found: {args.input}', file=sys.stderr)
        sys.exit(1)

    runs, stage_names, comm_names = collect_runs(args.input, args.twopc)
    if not runs:
        print('Warning: no complete ORQ logs found; nothing written.', file=sys.stderr)
        sys.exit(1)

    dynamic_fields = ([f'{n}(s)' for n in stage_names]
                      + [f'{n}_comm' for n in comm_names])

    if args.median:
        rows = median_per_query(runs, stage_names, comm_names)
        fieldnames = (['query', 'runs', 'sf', 'threads', 'overall_time(s)']
                      + dynamic_fields + ['preproc(s)', 'flag'])
    else:
        rows = [flatten(r, stage_names, comm_names)
                for r in sorted(runs, key=lambda r: (natural_key(r['query']), r['file']))]
        fieldnames = FIXED_FIELDS + dynamic_fields + TAIL_FIELDS

    with open(args.output, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames, extrasaction='ignore')
        writer.writeheader()
        for row in rows:
            writer.writerow({k: row.get(k, 'N/A') for k in fieldnames})

    print(f'Extracted {len(runs)} run(s), {len(rows)} row(s) -> {args.output}')
    for row in rows[:5]:
        print(f"  {row['query']}: overall={row['overall_time(s)']}s "
              f"sf={row['sf']}")
    if len(rows) > 5:
        print(f'  ... ({len(rows)} rows total)')


if __name__ == '__main__':
    main()

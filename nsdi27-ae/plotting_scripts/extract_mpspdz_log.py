#!/usr/bin/env python3
"""
Extract MP-SPDZ sort benchmark results from a log file into CSV.

Parses logs produced by scripts/fig10/fig10_mpspdz.sh (MP-SPDZ
compile-run.py output). Each benchmark size is introduced by a marker line:

    Exponent 16                      (fig10_mpspdz.sh)
    Iteration 0: Exponent 16         (older format, also accepted)

followed by MP-SPDZ timing lines:

    Spent 3.49 seconds (1295.9 MB, 13337 tuples) on the online phase \
and 9.21 seconds (2591.8 MB) on the preprocessing/offline phase.

The parenthesized MB value is the communication of that phase; a line
without the offline part (or without the MB value) is also accepted.
Repeated runs of the same exponent are averaged.

Output columns: exponent, rows, online_time(s), offline_time(s),
total_time(s), online_comm(MB), offline_comm(MB), total_comm(MB), runs.

Usage:
  python3 extract_mpspdz_log.py -i 3pc-mpspdz.txt -o fig10_mpspdz.csv
"""

import argparse
import csv
import os
import re
import sys
from collections import defaultdict

# "Exponent 16" or "Iteration 0: Exponent 16"
EXPONENT_RE = re.compile(r'^(?:Iteration \d+:\s+)?Exponent (\d+)\s*$')

# Full form: online + offline, each with optional "(X MB, ...)" comm.
TIME_RE = re.compile(
    r'Spent ([\d.]+) seconds(?: \(([\d.]+) MB[^)]*\))? .*?on the online phase'
    r' and ([\d.]+) seconds(?: \(([\d.]+) MB[^)]*\))? .*?on the preprocessing/offline phase'
)
# Fallback: online phase only.
ONLINE_RE = re.compile(
    r'Spent ([\d.]+) seconds(?: \(([\d.]+) MB[^)]*\))? .*?on the online phase'
)


def parse_mpspdz_log(input_file):
    """Parse the log; return {exponent: {'online': [...], 'offline': [...],
    'online_comm': [...], 'offline_comm': [...]}} of per-run values."""
    data = defaultdict(lambda: {'online': [], 'offline': [],
                                'online_comm': [], 'offline_comm': []})
    current_exponent = None

    with open(input_file, 'r', encoding='utf-8') as f:
        for line in f:
            exponent_match = EXPONENT_RE.match(line.strip())
            if exponent_match:
                current_exponent = int(exponent_match.group(1))
                continue

            if current_exponent is None:
                continue

            time_match = TIME_RE.search(line)
            if time_match:
                rec = data[current_exponent]
                rec['online'].append(float(time_match.group(1)))
                if time_match.group(2) is not None:
                    rec['online_comm'].append(float(time_match.group(2)))
                rec['offline'].append(float(time_match.group(3)))
                if time_match.group(4) is not None:
                    rec['offline_comm'].append(float(time_match.group(4)))
                continue

            online_match = ONLINE_RE.search(line)
            if online_match:
                rec = data[current_exponent]
                rec['online'].append(float(online_match.group(1)))
                if online_match.group(2) is not None:
                    rec['online_comm'].append(float(online_match.group(2)))

    return data


def average(values):
    return sum(values) / len(values) if values else None


def main():
    parser = argparse.ArgumentParser(
        description='Extract MP-SPDZ sort benchmark results from a log into CSV.',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument('-i', '--input', required=True, help='input MP-SPDZ log file')
    parser.add_argument('-o', '--output', default=None,
                        help='output CSV file (default: <input basename>.csv)')
    args = parser.parse_args()

    if args.output is None:
        base = os.path.splitext(os.path.basename(args.input))[0]
        args.output = base + '.csv'
    elif not args.output.endswith('.csv'):
        args.output += '.csv'

    try:
        data = parse_mpspdz_log(args.input)
    except FileNotFoundError:
        print(f'Error: input file not found: {args.input}', file=sys.stderr)
        sys.exit(1)

    if not data:
        print('Warning: no MP-SPDZ timing lines found; nothing written.', file=sys.stderr)
        sys.exit(1)

    fieldnames = ['exponent', 'rows', 'online_time(s)', 'offline_time(s)',
                  'total_time(s)', 'online_comm(MB)', 'offline_comm(MB)',
                  'total_comm(MB)', 'runs']
    rows = []
    for exponent in sorted(data):
        rec = data[exponent]
        online = average(rec['online'])
        offline = average(rec['offline'])
        online_comm = average(rec['online_comm'])
        offline_comm = average(rec['offline_comm'])
        rows.append({
            'exponent': exponent,
            'rows': 2 ** exponent,
            'online_time(s)': online,
            'offline_time(s)': offline if offline is not None else 0.0,
            'total_time(s)': (online or 0.0) + (offline or 0.0),
            'online_comm(MB)': online_comm if online_comm is not None else 'N/A',
            'offline_comm(MB)': offline_comm if offline_comm is not None else 'N/A',
            'total_comm(MB)': ((online_comm or 0.0) + (offline_comm or 0.0))
                              if online_comm is not None or offline_comm is not None else 'N/A',
            'runs': len(rec['online']),
        })

    with open(args.output, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print(f'Extracted {len(rows)} exponent(s) -> {args.output}')
    for row in rows:
        print(f"  2^{row['exponent']}: online={row['online_time(s)']:.3f}s "
              f"offline={row['offline_time(s)']:.3f}s "
              f"comm={row['total_comm(MB)']}MB ({row['runs']} run(s))")


if __name__ == '__main__':
    main()

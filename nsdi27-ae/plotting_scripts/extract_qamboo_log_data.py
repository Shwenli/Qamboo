#!/usr/bin/env python3
"""
Generic log-to-CSV extractor.

Built-in mode (default) recognizes the two standard Qamboo log metrics,
keyed by an arbitrary task name (e.g. "Q9", "Radix Sort 64-bit"):

    Total <task> execution time: <value><ms|s>
    <task> took: <value><ms|s>
    Total <task> Communication Sent <value> MB, Recv <value> MB

Options:
  --section NAMES   treat a line that consists solely of one of these names
                    (e.g. LAN, WAN, RDMA, TCP) as a section marker; the current
                    section is added as the first CSV column.
  --delta           communication values are cumulative per-party counters;
                    report the per-task actual traffic instead (sum of the
                    per-party increments between consecutive tasks, where the
                    parties of one task are the consecutive Comm lines that
                    follow its time line).
  --regex LABEL=PATTERN
                    custom mode: ignore the built-in metrics and emit one CSV
                    row per regex match. Capture groups become columns (named
                    groups keep their names). Repeatable.
  --labels NAMES    merge mode: give several -i log files and one label per
                    file; the tasks of all files are aligned into one pivoted
                    CSV whose columns match the plotting data files, e.g.
                    "Query,TCP,RDMA" for fig12.
  --metric M        which metric the merge-mode columns hold:
                    time (default), sent or recv.

Examples:
  python3 extract_log_data.py -i exp_1_wan_tpch.txt -o fig.csv
  python3 extract_log_data.py -i exp_5_sort_scale.txt --section LAN RDMA WAN TCP
  python3 extract_log_data.py -i exp_2_sort_mpspdz.txt --delta
  python3 extract_log_data.py -i log.txt --regex 'row=rows: (?P<rows>\d+)'
  python3 extract_log_data.py -i tcp.log rdma.log --labels TCP RDMA -o fig12.csv
"""

import argparse
import csv
import os
import re
import sys

TIME_RE = re.compile(r'(?:Total (.+?) execution time|(.+?) took):\s*([\d.]+)\s*(ms|s)\b')
COMM_RE = re.compile(r'Total (.+?) Communication Sent\s+([\d.]+)\s*MB(?:,\s*Recv\s+([\d.]+)\s*MB)?')

# tracing_subscriber-style prefix, e.g. "2026-02-20T10:48:58.267062Z  INFO "
PREFIX_RE = re.compile(r'^\d{4}-\d{2}-\d{2}T\S+\s+(?:TRACE|DEBUG|INFO|WARN|ERROR)\s+')

DEFAULT_SECTIONS = ['LAN', 'RDMA', 'WAN', 'TCP']


def natural_key(task):
    """Sort task names naturally: Q2 before Q10, numbers embedded in text."""
    return [int(t) if t.isdigit() else t.lower() for t in re.split(r'(\d+)', task)]


def parse_builtin(input_file, sections, delta):
    """Extract the built-in time/communication metrics.

    Returns (rows, fieldnames). rows is a list of dicts with keys:
    section, task, execution_time, communication_sent, communication_recv.
    """
    records = {}          # (section, task) -> dict
    current_section = None
    section_set = set(sections or [])

    # --delta bookkeeping: task name of the pending time line and the
    # per-party cumulative counters seen so far.
    pending_task = None
    prev_sent = []

    with open(input_file, 'r', encoding='utf-8') as f:
        for line in f:
            stripped = line.strip()

            if stripped in section_set:
                current_section = stripped
                continue

            line = PREFIX_RE.sub('', line)

            time_match = TIME_RE.search(line)
            if time_match:
                task = (time_match.group(1) or time_match.group(2)).strip()
                value = float(time_match.group(3))
                if time_match.group(4) == 'ms':
                    value /= 1000.0
                key = (current_section, task)
                rec = records.setdefault(key, {'section': current_section, 'task': task})
                rec['execution_time'] = value
                pending_task = key if delta else None
                continue

            comm_match = COMM_RE.search(line)
            if comm_match:
                task = comm_match.group(1).strip()
                sent = float(comm_match.group(2))
                recv = float(comm_match.group(3)) if comm_match.group(3) else None

                if delta:
                    # Cumulative counters: attribute consecutive Comm lines
                    # after a time line to the parties of that task.
                    key = pending_task if pending_task is not None else (current_section, task)
                    rec = records.setdefault(key, {'section': key[0], 'task': key[1]})
                    party_idx = len(rec.setdefault('_sent_acc', []))
                    rec['_sent_acc'].append(sent)
                    if party_idx < len(prev_sent):
                        rec['communication_sent'] = rec.get('communication_sent', 0.0) + sent - prev_sent[party_idx]
                        prev_sent[party_idx] = sent
                    else:
                        # First task: counters started at 0.
                        rec['communication_sent'] = rec.get('communication_sent', 0.0) + sent
                        prev_sent.append(sent)
                else:
                    key = (current_section, task)
                    rec = records.setdefault(key, {'section': current_section, 'task': task})
                    rec['communication_sent'] = sent
                    if recv is not None:
                        rec['communication_recv'] = recv
                continue

    fieldnames = []
    if sections:
        fieldnames.append('Section')
    fieldnames += ['Task', 'Execution Time (s)', 'Communication Sent (MB)', 'Communication Recv (MB)']

    rows = []
    for key in sorted(records, key=lambda k: (str(k[0]), natural_key(k[1]))):
        rec = records[key]
        row = {}
        if sections:
            row['Section'] = rec['section'] or ''
        row['Task'] = rec['task']
        row['Execution Time (s)'] = rec.get('execution_time', 'N/A')
        row['Communication Sent (MB)'] = rec.get('communication_sent', 'N/A')
        row['Communication Recv (MB)'] = rec.get('communication_recv', 'N/A')
        rows.append(row)
    return rows, fieldnames


def parse_custom(input_file, sections, regex_specs):
    """Custom regex mode: one row per match, capture groups as columns."""
    compiled = []
    fieldnames = []
    if sections:
        fieldnames.append('Section')
    for label, pattern in regex_specs:
        rx = re.compile(pattern)
        compiled.append((label, rx))
        names = list(rx.groupindex.keys())
        if names:
            fieldnames.extend(n for n in names if n not in fieldnames)
        else:
            fieldnames.extend(f'{label}_{i + 1}' for i in range(rx.groups))

    rows = []
    current_section = None
    section_set = set(sections or [])
    with open(input_file, 'r', encoding='utf-8') as f:
        for line in f:
            stripped = line.strip()
            if stripped in section_set:
                current_section = stripped
                continue
            line = PREFIX_RE.sub('', line)
            for label, rx in compiled:
                m = rx.search(line)
                if not m:
                    continue
                row = {}
                if sections:
                    row['Section'] = current_section or ''
                if rx.groupindex:
                    row.update(m.groupdict())
                else:
                    for i, g in enumerate(m.groups()):
                        row[f'{label}_{i + 1}'] = g
                rows.append(row)
                break
    return rows, fieldnames


METRIC_KEYS = {
    'time': 'Execution Time (s)',
    'sent': 'Communication Sent (MB)',
    'recv': 'Communication Recv (MB)',
}


def parse_merge(inputs, labels, delta, metric):
    """Pivot several logs into one CSV aligned by task name.

    Produces the layout of the plotting data files, e.g. "Query,TCP,RDMA".
    """
    metric_key = METRIC_KEYS[metric]
    per_file = []
    tasks = []
    for path in inputs:
        rows, _ = parse_builtin(path, None, delta)
        values = {}
        for row in rows:
            task = row['Task']
            values[task] = row[metric_key]
            if task not in tasks:
                tasks.append(task)
        per_file.append(values)

    fieldnames = ['Query'] + list(labels)
    out_rows = []
    for task in sorted(tasks, key=natural_key):
        row = {'Query': task}
        for label, values in zip(labels, per_file):
            row[label] = values.get(task, 'N/A')
        out_rows.append(row)
    return out_rows, fieldnames


def main():
    parser = argparse.ArgumentParser(
        description='Extract metrics from a Qamboo experiment log into CSV.',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument('-i', '--input', required=True, nargs='+',
                        help='input log file(s); several files trigger merge mode')
    parser.add_argument('-o', '--output', default=None,
                        help='output CSV file (default: <input basename>.csv, '
                             'or merged.csv in merge mode)')
    parser.add_argument('--section', nargs='*', default=None,
                        help='bare-line section markers, e.g. --section LAN WAN '
                             '(default: %(default)s when flag given without names)')
    parser.add_argument('--delta', action='store_true',
                        help='treat communication counters as cumulative and '
                             'report per-task increments (multi-party logs)')
    parser.add_argument('--regex', action='append', default=[], metavar='LABEL=PATTERN',
                        help='custom extraction regex; repeatable. '
                             'Capture groups become columns.')
    parser.add_argument('--labels', nargs='+', default=None, metavar='NAME',
                        help='column names for merge mode, one per input file, '
                             'e.g. --labels TCP RDMA (default: file basenames)')
    parser.add_argument('--metric', choices=['time', 'sent', 'recv'], default='time',
                        help='metric used for the merge-mode columns (default: %(default)s)')

    args = parser.parse_args()

    merge_mode = len(args.input) > 1 or args.labels is not None

    if args.output is None:
        if merge_mode:
            args.output = 'merged.csv'
        else:
            base = os.path.splitext(os.path.basename(args.input[0]))[0]
            args.output = base + '.csv'
    elif not args.output.endswith('.csv'):
        args.output += '.csv'

    # --section with no names uses the defaults; flag absent means no sections.
    if args.section is not None and len(args.section) == 0:
        args.section = DEFAULT_SECTIONS

    try:
        if merge_mode:
            if args.regex or args.section:
                parser.error('merge mode (multiple -i / --labels) cannot be '
                             'combined with --regex or --section')
            labels = args.labels or [os.path.splitext(os.path.basename(p))[0]
                                     for p in args.input]
            if len(labels) != len(args.input):
                parser.error(f'--labels expects one name per input file '
                             f'({len(args.input)} files, {len(labels)} labels)')
            rows, fieldnames = parse_merge(args.input, labels, args.delta, args.metric)
        elif args.regex:
            specs = []
            for spec in args.regex:
                if '=' not in spec:
                    parser.error(f'--regex expects LABEL=PATTERN, got: {spec!r}')
                specs.append(tuple(spec.split('=', 1)))
            rows, fieldnames = parse_custom(args.input[0], args.section, specs)
        else:
            rows, fieldnames = parse_builtin(args.input[0], args.section, args.delta)
    except FileNotFoundError as e:
        print(f'Error: input file not found: {e.filename}', file=sys.stderr)
        sys.exit(1)

    if not rows:
        print('Warning: no matching lines found; nothing written.', file=sys.stderr)
        sys.exit(1)

    with open(args.output, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames, extrasaction='ignore')
        writer.writeheader()
        for row in rows:
            writer.writerow({k: row.get(k, 'N/A') for k in fieldnames})

    print(f'Extracted {len(rows)} row(s) -> {args.output}')
    for row in rows[:5]:
        print('  ' + ', '.join(str(row.get(k, '')) for k in fieldnames))
    if len(rows) > 5:
        print(f'  ... ({len(rows)} rows total)')


if __name__ == '__main__':
    main()

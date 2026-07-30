#!/usr/bin/env python3
"""
Analyze thread scaling experiment results and generate summary.

Usage:
    python3 analyze_results.py <log_file>
    
Example:
    python3 analyze_results.py ../../../../experiments/result/thread_scaling/thread_scaling_sf1.log
"""

import sys
import re
import json
from collections import defaultdict
import os

def parse_log(log_file):
    """Parse the thread scaling log file and extract results."""
    results = defaultdict(lambda: defaultdict(dict))
    
    current_rayon = None
    current_comm = None
    current_query = None
    
    with open(log_file, 'r') as f:
        for line in f:
            line = line.strip()
            
            # Parse RAYON_NUM_THREADS and NUM_COMMTHREADS
            match = re.search(r'RAYON_NUM_THREADS=(\d+),\s*NUM_COMMTHREADS=(\d+)', line)
            if match:
                current_rayon = int(match.group(1))
                current_comm = int(match.group(2))
                continue
            
            # Parse query number
            match = re.search(r'^Q(\d+):', line)
            if match:
                current_query = f"Q{match.group(1)}"
                continue
            
            # Parse timing info
            match = re.search(r'INFO.*Total.*time:\s*([\d.]+)s', line, re.IGNORECASE)
            if match and current_rayon is not None and current_comm is not None and current_query is not None:
                time_val = float(match.group(1))
                results[current_query][(current_rayon, current_comm)] = time_val
                continue
            
            # Check for failures
            if line.endswith('FAILED'):
                if current_query and current_rayon is not None and current_comm is not None:
                    results[current_query][(current_rayon, current_comm)] = None
    
    return results

def find_best_config(results):
    """Find the best thread configuration for each query."""
    best_configs = {}
    
    for query, configs in results.items():
        valid_configs = {k: v for k, v in configs.items() if v is not None}
        if valid_configs:
            best = min(valid_configs.items(), key=lambda x: x[1])
            best_configs[query] = {
                'rayon': best[0][0],
                'comm': best[0][1],
                'time': best[1]
            }
    
    return best_configs

def print_summary(results, best_configs):
    """Print a formatted summary of results."""
    print("=" * 80)
    print("Thread Scaling Analysis Results")
    print("=" * 80)
    print()
    
    # Best configurations
    print("Best Configurations (Minimum Time):")
    print("-" * 80)
    print(f"{'Query':<10} {'RAYON_THREADS':<15} {'COMM_THREADS':<15} {'Time (s)':<15}")
    print("-" * 80)
    
    for query in sorted(best_configs.keys()):
        cfg = best_configs[query]
        print(f"{query:<10} {cfg['rayon']:<15} {cfg['comm']:<15} {cfg['time']:<15.2f}")
    
    print()
    
    # Speedup analysis
    print("Speedup Analysis (vs Single-Threaded):")
    print("-" * 80)
    print(f"{'Query':<10} {'Base Time (1,1)':<20} {'Best Time':<20} {'Speedup':<15}")
    print("-" * 80)
    
    for query in sorted(results.keys()):
        base_time = results[query].get((1, 1))
        if base_time and query in best_configs:
            best_time = best_configs[query]['time']
            speedup = base_time / best_time
            print(f"{query:<10} {base_time:<20.2f} {best_time:<20.2f} {speedup:<15.2f}x")
    
    print()
    
    # Detailed results per query
    print("Detailed Results (Time in seconds):")
    print("=" * 80)
    
    for query in sorted(results.keys()):
        print()
        print(f"{query}:")
        print("-" * 60)
        
        configs = results[query]
        if not configs:
            print("  No results found")
            continue
        
        # Get unique values
        rayon_values = sorted(set(k[0] for k in configs.keys()))
        comm_values = sorted(set(k[1] for k in configs.keys()))
        
        # Print header
        header = f"{'RAYON\\COMM':<12}"
        for comm in comm_values:
            header += f"{comm:<12}"
        print(header)
        print("-" * 60)
        
        # Print rows
        for rayon in rayon_values:
            row = f"{rayon:<12}"
            for comm in comm_values:
                time_val = configs.get((rayon, comm))
                if time_val is not None:
                    row += f"{time_val:<12.2f}"
                else:
                    row += f"{'FAIL':<12}"
            print(row)

def export_json(results, output_file):
    """Export results to JSON format."""
    # Convert tuple keys to strings for JSON
    json_results = {}
    for query, configs in results.items():
        json_results[query] = {
            f"rayon_{k[0]}_comm_{k[1]}": v for k, v in configs.items()
        }
    
    with open(output_file, 'w') as f:
        json.dump(json_results, f, indent=2)
    
    print(f"Results exported to: {output_file}")

def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <log_file> [output.json]")
        print()
        print("Example:")
        print(f"  {sys.argv[0]} ../../../../experiments/result/thread_scaling/thread_scaling_sf1.log")
        print(f"  {sys.argv[0]} ../../../../experiments/result/thread_scaling/thread_scaling_sf1.log results.json")
        sys.exit(1)
    
    log_file = sys.argv[1]
    
    if not os.path.exists(log_file):
        print(f"Error: Log file not found: {log_file}")
        sys.exit(1)
    
    # Parse and analyze
    results = parse_log(log_file)
    best_configs = find_best_config(results)
    
    # Print summary
    print_summary(results, best_configs)
    
    # Export to JSON if requested
    if len(sys.argv) >= 3:
        export_json(results, sys.argv[2])

if __name__ == "__main__":
    main()

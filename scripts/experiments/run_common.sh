# shellcheck shell=bash
# ==============================================================================
# run_common.sh — Shared engine for the Qamboo experiment runners
#   (run_tpch.sh, run_secrecy.sh, run_operator.sh, run_optimization.sh).
#   Not meant to be executed directly; sourced by the suite scripts.
#
# Provides:
#   parse_common_opts  — parses -m/-t/-s/-n/-h/--start/--end/--rayon/--log/--no-log
#   expand_targets     — expands "all", comma lists and "1..8" ranges
#   contains           — membership test
#   setup_log          — resolves the stat log path ($1 = result subdir)
#   run_one            — builds and runs ONE binary with 3 parties
#   run_all            — iterates TARGETS/BINS, logs "INFO Total" lines
#
# Contract: the sourcing script sets TARGETS and BINS (parallel arrays)
# before calling run_all, and may override defaults (THREADS, SF, ROWS, START, END).
#
# Hosts: -h takes a comma-separated list of 3 hosts, one per party
# (party i runs on the i-th host). A host equal to $(hostname),
# "localhost" or "127.0.0.1" runs as a local process; all others run
# via SSH. The local machine does not have to be one of the parties.
# ==============================================================================

COMMON_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if ! PROJECT_ROOT="$(git -C "$COMMON_DIR" rev-parse --show-toplevel 2>/dev/null)"; then
    PROJECT_ROOT="$(cd "$COMMON_DIR/../.." && pwd)"
fi

# Defaults (suite scripts may override after parsing)
MODE="local"
THREADS=""
SF=""
ROWS=""
START=20
END=26
HOST_LIST="node0,node1,node2"
RAYON=""
LOG_OVERRIDE=""
NO_LOG=false

parse_common_opts() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -m)       MODE="$2"; shift 2 ;;
            -t)       THREADS="$2"; shift 2 ;;
            -s)       SF="$2"; shift 2 ;;
            -n)       ROWS="$2"; shift 2 ;;
            --start)  START="$2"; shift 2 ;;
            --end)    END="$2"; shift 2 ;;
            -h)       HOST_LIST="$2"; shift 2 ;;
            --rayon)  RAYON="$2"; shift 2 ;;
            --log)    LOG_OVERRIDE="$2"; shift 2 ;;
            --no-log) NO_LOG=true; shift ;;
            *) echo "Error: unknown option '$1'" >&2; return 1 ;;
        esac
    done
    case "$MODE" in
        local|tcp|rdma) ;;
        *) echo "Error: mode must be local|tcp|rdma (got '$MODE')" >&2; return 1 ;;
    esac
}

# Expand "all", comma lists ("1,3,5") and numeric ranges ("1..8").
# Usage: expand_targets <input> [all-values...]
expand_targets() {
    local input="$1"; shift
    local -a all_vals=("$@")
    local -a out=()
    if [ "$input" == "all" ]; then
        out=("${all_vals[@]}")
    else
        local part start end i
        IFS=',' read -r -a parts <<< "$input"
        for part in "${parts[@]}"; do
            if [[ "$part" == *".."* ]]; then
                start=$(echo "$part" | cut -d'.' -f1)
                end=$(echo "$part" | cut -d'.' -f3)
                for ((i=start; i<=end; i++)); do out+=("$i"); done
            else
                out+=("$part")
            fi
        done
    fi
    echo "${out[@]}"
}

contains() {
    local x="$1"; shift
    local y
    for y in "$@"; do [ "$x" == "$y" ] && return 0; done
    return 1
}

# Resolve LOG_FILE. $1 = subdir under experiments/result/ (e.g. tpch_query).
# Modes: local -> local/stat_output.log; tcp -> multinode/stat_output.log;
# rdma -> multinode/stat_output_rdma.log.
setup_log() {
    local mode_dir log_name="stat_output.log"
    if [ "$MODE" == "local" ]; then mode_dir="local"; else mode_dir="multinode"; fi
    [ "$MODE" == "rdma" ] && log_name="stat_output_rdma.log"
    if [ -n "$LOG_OVERRIDE" ]; then
        LOG_FILE="$LOG_OVERRIDE"
    else
        LOG_FILE="$PROJECT_ROOT/experiments/result/$1/$mode_dir/$log_name"
    fi
    $NO_LOG || mkdir -p "$(dirname "$LOG_FILE")"
}

# True if $1 refers to this machine.
is_local_host() {
    local h="$1"
    [ "$h" == "$(hostname)" ] || [ "$h" == "localhost" ] || [ "$h" == "127.0.0.1" ]
}

# Build and run ONE binary with 3 parties.
# Uses globals: MODE THREADS SF START END HOST_LIST RAYON PROJECT_ROOT.
run_one() {
    local bin="$1"
    local bin_path="./target/release/$bin"

    # Extra CLI args depend on the binary
    local extra_args
    case "$bin" in
        multi_keys_join)        extra_args="" ;;
        mul_bench|compare_bench) extra_args="-t $THREADS" ;;
        radix_sort_mpspdz)      extra_args="-t $THREADS" ;;
        radix_sort_scalability) extra_args="-t $THREADS --start $START --end $END" ;;
        *)                      extra_args="-t $THREADS -s $SF" ;;
    esac

    # Optional Rayon compute-thread setting (must also apply on remote parties)
    local rayon_prefix=""
    [ -n "$RAYON" ] && rayon_prefix="RAYON_NUM_THREADS=$RAYON"

    echo ">>> [Build] $bin"
    if ! (cd "$PROJECT_ROOT" && RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin "$bin" --features tcp); then
        echo "Error: build failed for '$bin'; skipping this target." >&2
        return 1
    fi

    cd "$PROJECT_ROOT"

    if [ "$MODE" == "local" ]; then
        local cfg="experiments/net/local/"
        [ -n "$RAYON" ] && export RAYON_NUM_THREADS="$RAYON"
        $bin_path -c "$cfg" -p 0 $extra_args &
        $bin_path -c "$cfg" -p 1 $extra_args &
        $bin_path -c "$cfg" -p 2 $extra_args &
        wait
    else
        local cfg="experiments/net/multinode/"
        if [ "$MODE" == "rdma" ]; then bin_path="smc_run $bin_path"; fi

        # Party i runs on the i-th host of -h (local hosts run directly,
        # remote hosts via SSH)
        local -a HOSTS
        IFS=',' read -r -a HOSTS <<< "$HOST_LIST"
        if [ ${#HOSTS[@]} -ne 3 ]; then
            echo "Error: -h expects exactly 3 hosts, one per party (got '$HOST_LIST')" >&2
            return 1
        fi

        local h
        for h in "${HOSTS[@]}"; do
            is_local_host "$h" || scp "target/release/$bin" "$h:$PROJECT_ROOT/target/release/"
        done

        local p
        for p in 0 1 2; do
            h="${HOSTS[$p]}"
            if is_local_host "$h"; then
                $rayon_prefix $bin_path -c "$cfg" -p "$p" $extra_args &
            else
                ssh "$h" "cd $PROJECT_ROOT && $rayon_prefix $bin_path -c $cfg -p $p $extra_args" &
            fi
        done
        wait
    fi
}

# Iterate TARGETS/BINS, run each, append "INFO Total" lines to LOG_FILE.
run_all() {
    local label="$1"

    echo "============================================================"
    echo "Qamboo Experiment Runner: $label"
    echo "Mode: $MODE | Targets: ${TARGETS[*]}"
    echo "Threads: $THREADS | SF/Shift: $SF${ROWS:+ | Rows: $ROWS}${RAYON:+ | Rayon: $RAYON}"
    if [ "$MODE" != "local" ]; then echo "Hosts: $HOST_LIST"; fi
    $NO_LOG || echo "Log: $LOG_FILE"
    echo "============================================================"

    local -a FAILED=()
    local i target bin
    for i in "${!TARGETS[@]}"; do
        target="${TARGETS[$i]}"
        bin="${BINS[$i]}"
        echo ""
        echo ">>> Running '$target' (bin: $bin, mode: $MODE) ..."

        # Note: $? after a pipeline is tee's status, so capture run_one's
        # real exit code via PIPESTATUS.
        local rc
        if $NO_LOG; then
            run_one "$bin" 2>&1
            rc=$?
        else
            run_one "$bin" 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
            rc=${PIPESTATUS[0]}
        fi

        if [ $rc -eq 0 ]; then
            echo ">>> Target '$target' Finished Successfully."
        else
            echo ">>> Target '$target' Failed."
            FAILED+=("$target")
        fi

        # Cool down to ensure ports are released
        if [ "$MODE" == "local" ]; then sleep 1; else sleep 2; fi
    done

    echo ""
    echo "============================================================"
    if [ ${#FAILED[@]} -eq 0 ]; then
        echo "All Experiments Completed Successfully."
    else
        echo "Completed with failures: ${FAILED[*]}"
    fi
    echo "============================================================"

    [ ${#FAILED[@]} -eq 0 ]
}

#!/bin/bash

# ==============================================================================
# Secrecy Paper Experiments - Multinode Version (Including TPCH Queries)
# ==============================================================================
# This script runs Secrecy and TPCH queries with the exact scale factors used in the paper.
# 
# Usage: ./run_multinode_exp_paper.sh
# 
# Query configurations:
# Secrecy Queries:
# - Comorbidity (exp_q1): first table 2m, second table 256
#   cohortSize(scaleFactor) -> 256
#   diagnosisSize(scaleFactor) -> 2097152
#   SF = 1
#
# - Aspirin (exp_q3): input tables 32k
#   getDiagnosisTable::size_t S -> 32768;
#   getMedicationTable::size_t S -> 32768;
#   ORQ SF1 = 2M for the smaller table.
#   SF = 0.016384
#
# - Rcdiff (exp_q2): first table 2m
#   getDiagnosisTable::size_t S -> 2097152;
#   ORQ SF1 = 3M, so run at SF 0.699
#   SF = 0.69905067
#
# - Credit (exp_qcredit): input table 2m
#   getCreditScoreTable::size_t S -> 2097152;
#   ORQ SF1 = 5M
#   SF = 0.4194304
#
# - Password (exp_qpwd): input table 2m
#   getPasswordTable::size_t S -> 2097152;
#   ORQ SF1 = 5M
#   SF = 0.4194304
#
# TPCH Queries:
# - Q13 (exp_tpch_q13): Orders 256k, customers 32k
#   customersSize() -> 32768
#   ordersSize() -> 262144
#   SF = 0.17476267
#
# - Q4 (exp_tpch_q4): Lineitems 128k, orders 32k
#   ordersSize() -> 32768
#   ORQ SF1 = 7.5M combined
#   SF = 0.02184533
#
# - Q6 (exp_tpch_q6): 8m
#   ordersSize() -> 2097152
#   ORQ SF1 = 6M
#   SF = 1.39810133
# ==============================================================================

set -o pipefail

# Fixed parameters
NUM_THREADS=16
MULTINODE_SCRIPT_DIR="./multinode"

# Use absolute path for safety
CURRENT_DIR=$(pwd)

# Log file location
LOG_FILE="$CURRENT_DIR/../../../experiments/result/secrecy_query/multinode/stat_output_paper.log"
mkdir -p "$(dirname "$LOG_FILE")"

# Check if multinode directory exists
if [ ! -d "$MULTINODE_SCRIPT_DIR" ]; then
    echo "Error: Directory '$MULTINODE_SCRIPT_DIR' not found."
    exit 1
fi

echo "============================================================"
echo "Secrecy Paper Experiments - Multinode"
echo "Threads: $NUM_THREADS (fixed)"
echo "============================================================"

# ==============================================================================
# Query 1: Comorbidity (exp_q1)
# SF = 1
# ==============================================================================
echo ""
echo ">>> Running Comorbidity (exp_q1) - SF=1 ..."
SCRIPT_PATH="$MULTINODE_SCRIPT_DIR/run_comorbidity_ssh.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    (cd "$MULTINODE_SCRIPT_DIR" && ./run_comorbidity_ssh.sh "$NUM_THREADS" "1") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
    if [ $? -eq 0 ]; then
        echo ">>> Comorbidity Finished Successfully."
    else
        echo ">>> Comorbidity Failed."
    fi
else
    echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
fi
sleep 2

# ==============================================================================
# Query 2: Aspirin (exp_q3)
# SF = 0.016384
# ==============================================================================
echo ""
echo ">>> Running Aspirin (exp_q3) - SF=0.016384 ..."
SCRIPT_PATH="$MULTINODE_SCRIPT_DIR/run_aspirin_ssh.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    (cd "$MULTINODE_SCRIPT_DIR" && ./run_aspirin_ssh.sh "$NUM_THREADS" "0.016384") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
    if [ $? -eq 0 ]; then
        echo ">>> Aspirin Finished Successfully."
    else
        echo ">>> Aspirin Failed."
    fi
else
    echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
fi
sleep 2

# ==============================================================================
# Query 3: Rcdiff (exp_q2)
# SF = 0.69905067
# ==============================================================================
echo ""
echo ">>> Running Rcdiff (exp_q2) - SF=0.69905067 ..."
SCRIPT_PATH="$MULTINODE_SCRIPT_DIR/run_rcdiff_ssh.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    (cd "$MULTINODE_SCRIPT_DIR" && ./run_rcdiff_ssh.sh "$NUM_THREADS" "0.69905067") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
    if [ $? -eq 0 ]; then
        echo ">>> Rcdiff Finished Successfully."
    else
        echo ">>> Rcdiff Failed."
    fi
else
    echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
fi
sleep 2

# ==============================================================================
# Query 4: Credit (exp_qcredit)
# SF = 0.4194304
# ==============================================================================
echo ""
echo ">>> Running Credit (exp_qcredit) - SF=0.4194304 ..."
SCRIPT_PATH="$MULTINODE_SCRIPT_DIR/run_credit_ssh.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    (cd "$MULTINODE_SCRIPT_DIR" && ./run_credit_ssh.sh "$NUM_THREADS" "0.4194304") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
    if [ $? -eq 0 ]; then
        echo ">>> Credit Finished Successfully."
    else
        echo ">>> Credit Failed."
    fi
else
    echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
fi
sleep 2

# ==============================================================================
# Query 5: Password (exp_qpwd)
# SF = 0.4194304
# ==============================================================================
echo ""
echo ">>> Running Password (exp_qpwd) - SF=0.4194304 ..."
SCRIPT_PATH="$MULTINODE_SCRIPT_DIR/run_pwd_ssh.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    (cd "$MULTINODE_SCRIPT_DIR" && ./run_pwd_ssh.sh "$NUM_THREADS" "0.4194304") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
    if [ $? -eq 0 ]; then
        echo ">>> Password Finished Successfully."
    else
        echo ">>> Password Failed."
    fi
else
    echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
fi
sleep 2

# ==============================================================================
# Query 6: TPCH Q13 (exp_tpch_q13)
# Orders 256k, customers 32k
# SF = 0.17476267
# ==============================================================================
echo ""
echo ">>> Running TPCH Q13 (exp_tpch_q13) - SF=0.17476267 ..."
SCRIPT_PATH="../tpch/multinode/run_q13_ssh.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    (cd "../tpch/multinode" && ./run_q13_ssh.sh "$NUM_THREADS" "0.17476267") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
    if [ $? -eq 0 ]; then
        echo ">>> TPCH Q13 Finished Successfully."
    else
        echo ">>> TPCH Q13 Failed."
    fi
else
    echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
fi
sleep 2

# ==============================================================================
# Query 7: TPCH Q4 (exp_tpch_q4)
# Lineitems 128k, orders 32k
# SF = 0.02184533
# ==============================================================================
echo ""
echo ">>> Running TPCH Q4 (exp_tpch_q4) - SF=0.02184533 ..."
SCRIPT_PATH="../tpch/multinode/run_q4_ssh.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    (cd "../tpch/multinode" && ./run_q4_ssh.sh "$NUM_THREADS" "0.02184533") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
    if [ $? -eq 0 ]; then
        echo ">>> TPCH Q4 Finished Successfully."
    else
        echo ">>> TPCH Q4 Failed."
    fi
else
    echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
fi
sleep 2

# ==============================================================================
# Query 8: TPCH Q6 (exp_tpch_q6)
# SF = 1.39810133
# ==============================================================================
echo ""
echo ">>> Running TPCH Q6 (exp_tpch_q6) - SF=1.39810133 ..."
SCRIPT_PATH="../tpch/multinode/run_q6_ssh.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    (cd "../tpch/multinode" && ./run_q6_ssh.sh "$NUM_THREADS" "1.39810133") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
    if [ $? -eq 0 ]; then
        echo ">>> TPCH Q6 Finished Successfully."
    else
        echo ">>> TPCH Q6 Failed."
    fi
else
    echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
fi
sleep 2

echo ""
echo "============================================================"
echo "All Paper Experiments Completed (Secrecy + TPCH)."
echo "Results logged to: $LOG_FILE"
echo "============================================================"

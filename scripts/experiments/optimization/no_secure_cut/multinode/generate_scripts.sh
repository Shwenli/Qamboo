#!/bin/bash

# Template files
SSH_TEMPLATE="run_q2_no_secure_cut_ssh.sh"
RDMA_TEMPLATE="run_q2_no_secure_cut_ssh_rdma.sh"

if [ ! -f "$SSH_TEMPLATE" ] || [ ! -f "$RDMA_TEMPLATE" ]; then
    echo "Error: Templates run_q2_no_secure_cut_ssh.sh or run_q2_no_secure_cut_ssh_rdma.sh not found."
    exit 1
fi

echo "Generating scripts for TPC-H queries 1-22..."

for i in {2,3,5,8,13,17,18,20,21}; do
    # Skip if it's the template itself (though sed would handle it fine, optimizing)
    if [ "$i" -eq 2 ]; then
        continue
    fi

    # Generate SSH script
    OUT_SSH="run_q${i}_no_secure_cut_ssh.sh"
    sed "s/q2/q$i/g" "$SSH_TEMPLATE" > "$OUT_SSH"
    chmod +x "$OUT_SSH"
    echo "Created $OUT_SSH"

    # Generate RDMA script
    OUT_RDMA="run_q${i}_no_secure_cut_ssh_rdma.sh"
    sed "s/q2/q$i/g" "$RDMA_TEMPLATE" > "$OUT_RDMA"
    chmod +x "$OUT_RDMA"
    echo "Created $OUT_RDMA"
done

echo "Done."

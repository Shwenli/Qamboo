#!/usr/bin/env bash

# We need this script on Aliyun to make sure the machines know each other's IP addresses.
# ./_update_hostfile.sh -x node -i 

usage () {
    echo "Usage: ${0} [options]"
    echo "Example: ${0} -x node -i 1.0.0.1,1.0.0.2"
    echo "OPTIONS:"
    echo "  [-h]                                    Show this help"
    echo "  [-i]                                    List of IP addresses of the remote nodes."
    echo "  [-x node prefix]                        Prefix for remote nodes. Default is 'node'."
    exit 1
}

(( $# < 1 )) && usage

# Defaults
node_prefix="node"

while getopts "hi:x:" opt; do
    case ${opt} in
        h)
            usage
            ;;
        i)
            ip_addresses=(${OPTARG})
            ;;
        x)
            node_prefix=${OPTARG}
            ;;
        \?)
            echo "Invalid option: $OPTARG" 1>&2
            ;;
    esac
done

# Update /etc/hosts non-destructively: node entries live in a marked block
# that this script owns. Everything else in the file (e.g. the cloud image's
# default hostname entry) is preserved across runs.
HOSTS_BEGIN="# >>> Qamboo cluster nodes >>>"
HOSTS_END="# <<< Qamboo cluster nodes <<<"

# Write IP addresses to /etc/hosts with the node prefix + 0-index
# For example, ip_addresses = "18.188.23.207,18.188.23.212"
# First split the string into an array of IP addresses
ip_addresses_list=$(echo $ip_addresses | tr "," "\n")

# Remove the previously managed block (if any), leaving the rest of
# /etc/hosts untouched. A .bak backup is kept alongside.
sudo sed -i.bak "/${HOSTS_BEGIN}/,/${HOSTS_END}/d" /etc/hosts

# Then append the fresh block such as:
# "18.188.23.207 node0"
# "18.188.23.212 node1"
{
    echo "${HOSTS_BEGIN}"
    index=0
    for ip in $ip_addresses_list; do
        echo "$ip $node_prefix$index"
        index=$((index+1))
    done
    echo "${HOSTS_END}"
} | sudo tee -a /etc/hosts > /dev/null

index=0
for ip in $ip_addresses_list; do
    ssh-keygen -R $node_prefix$index
    index=$((index+1))
done


# Apply the same block update on every other node: remove their old managed
# block and append the fresh one, instead of overwriting the whole
# /etc/hosts with this node's version (each node keeps its own entries,
# e.g. the cloud image's hostname mapping).
BLOCK="$(sed -n "/${HOSTS_BEGIN}/,/${HOSTS_END}/p" /etc/hosts)"
index=0
for ip in $ip_addresses_list; do
    # Skip the local node: its /etc/hosts was already updated above.
    # (Convention, same as deploy.sh: the first IP in the list is node0,
    # the machine this script runs on.)
    if [[ $index -eq 0 ]]; then
        index=$((index+1))
        continue
    fi
    ssh -o StrictHostKeyChecking=no "$ip" \
        "sudo sed -i.bak '/${HOSTS_BEGIN}/,/${HOSTS_END}/d' /etc/hosts && echo '${BLOCK}' | sudo tee -a /etc/hosts > /dev/null"
    index=$((index+1))
done
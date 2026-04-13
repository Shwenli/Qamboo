#!/bin/bash
set -e

cd "$(dirname "$0")"

# 检查匿名仓库 remote 是否存在
if ! git remote | grep -q "^anonymous$"; then
    echo "Error: 'anonymous' remote not found."
    echo "Please add it first:"
    echo "  git remote add anonymous https://github.com/anonymous-qamboo/Qamboo.git"
    exit 1
fi

echo "=== Generating documentation ==="
./gen-docs.sh

echo ""
echo "=== Committing changes ==="
git add docs/
git diff --cached --quiet || git commit -m "Update documentation"

echo ""
echo "=== Pushing to main repository ==="
git push origin main

echo ""
echo "=== Pushing to anonymous repository ==="
git push anonymous main

echo ""
echo "✓ Sync completed!"
echo "  Main: $(git remote get-url origin)"
echo "  Anonymous: $(git remote get-url anonymous)"

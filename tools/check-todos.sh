#!/bin/bash
# Check that TODO/FIXME comments include issue references
# Part of BUNKER MINER pre-commit security checks

set -euo pipefail

EXIT_CODE=0

echo "🔍 Checking TODO/FIXME comments for issue references..."

# Find TODO/FIXME comments without issue references
if git diff --cached --name-only | grep -E '\.(rs|cpp|hpp|c|h|py|js|ts)$' | xargs grep -Hn "TODO\|FIXME" | grep -v "#[0-9]" | grep -v "TODO(" | head -10; then
    echo "❌ Found TODO/FIXME comments without issue references!"
    echo "Please include issue numbers like: TODO(#123) or FIXME: See issue #456"
    echo "This helps track technical debt and ensures items aren't forgotten."
    EXIT_CODE=1
fi

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ All TODO/FIXME comments have proper issue references"
fi

exit $EXIT_CODE
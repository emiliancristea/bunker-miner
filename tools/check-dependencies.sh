#!/bin/bash
# Check that dependency versions are properly pinned
# Part of BUNKER MINER pre-commit security checks

set -euo pipefail

EXIT_CODE=0

echo "🔍 Checking dependency version pinning..."

# Check Cargo.toml files
CARGO_FILES=$(git diff --cached --name-only | grep "Cargo\.toml$" || true)
if [ -n "$CARGO_FILES" ]; then
    for file in $CARGO_FILES; do
        echo "Checking Rust dependencies in: $file"
        
        # Check for unpinned versions (using "*" or caret ranges without specific version)
        if git show ":$file" | grep -E '^\s*[a-zA-Z0-9_-]+\s*=\s*"\*"'; then
            echo "❌ Found unpinned dependency in: $file"
            echo "   Use specific version numbers instead of '*'"
            EXIT_CODE=1
        fi
        
        # Warn about caret dependencies (allow but warn)
        if git show ":$file" | grep -E '^\s*[a-zA-Z0-9_-]+\s*=\s*"\^'; then
            echo "⚠️  Found caret dependency in: $file"
            echo "   Consider using exact versions for better reproducibility"
        fi
    done
fi

# Check CMakeLists.txt files
CMAKE_FILES=$(git diff --cached --name-only | grep "CMakeLists\.txt$" || true)
if [ -n "$CMAKE_FILES" ]; then
    for file in $CMAKE_FILES; do
        echo "Checking CMake dependencies in: $file"
        
        # Check for find_package without version requirements
        if git show ":$file" | grep -E "find_package\s*\(\s*[A-Za-z0-9_]+\s*REQUIRED\s*\)" | grep -v "VERSION"; then
            echo "⚠️  Found unversioned find_package in: $file"
            echo "   Consider specifying minimum required versions"
        fi
    done
fi

# Check package.json files (if any)
PACKAGE_FILES=$(git diff --cached --name-only | grep "package\.json$" || true)
if [ -n "$PACKAGE_FILES" ]; then
    for file in $PACKAGE_FILES; do
        echo "Checking Node.js dependencies in: $file"
        
        # Check for unpinned versions
        if git show ":$file" | grep -E '"\s*\^\s*|\s*~\s*|\s*\*\s*"'; then
            echo "⚠️  Found flexible dependency versions in: $file"
            echo "   Consider using exact versions for better reproducibility"
        fi
    done
fi

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ Dependency pinning check passed"
else
    echo ""
    echo "📦 Dependency Check Failed!"
    echo "   Unpinned dependencies can lead to unreproducible builds"
    echo "   and potential security issues from unexpected updates."
fi

exit $EXIT_CODE
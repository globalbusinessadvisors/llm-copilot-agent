#!/bin/bash
# Test Implementation Verification Script
# Verifies that all test files and infrastructure are in place

set -e

echo "🔍 Verifying LLM-CoPilot-Agent Test Implementation..."
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0

# Check function
check_file() {
    local file=$1
    local description=$2

    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $description"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $description (missing: $file)"
        ((FAILED++))
    fi
}

check_directory() {
    local dir=$1
    local description=$2

    if [ -d "$dir" ]; then
        echo -e "${GREEN}✓${NC} $description"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $description (missing: $dir)"
        ((FAILED++))
    fi
}

echo "📁 Checking Directory Structure..."
check_directory "tests/integration" "Integration tests directory"
check_directory "tests/common" "Common utilities directory"
check_directory "benches" "Benchmarks directory"
check_directory ".github/workflows" "GitHub workflows directory"
echo ""

echo "🧪 Checking Integration Test Files..."
check_file "tests/integration/mod.rs" "Integration module"
check_file "tests/integration/api_tests.rs" "API tests (23 tests)"
check_file "tests/integration/conversation_tests.rs" "Conversation tests (20 tests)"
check_file "tests/integration/workflow_tests.rs" "Workflow tests (24 tests)"
echo ""

echo "🛠️  Checking Test Utility Files..."
check_file "tests/common/mod.rs" "Common module"
check_file "tests/common/fixtures.rs" "Test fixtures"
check_file "tests/common/mocks.rs" "Mock implementations"
check_file "tests/common/assertions.rs" "Custom assertions"
check_file "tests/common/database.rs" "Database helpers"
echo ""

echo "⚡ Checking Benchmark Files..."
check_file "benches/benchmarks.rs" "Performance benchmarks"
echo ""

echo "🔧 Checking Build Automation..."
check_file "Makefile" "Makefile with test targets"
echo ""

echo "🤖 Checking CI/CD Configuration..."
check_file ".github/workflows/ci.yml" "GitHub Actions CI workflow"
echo ""

echo "📚 Checking Documentation..."
check_file "tests/README.md" "Test suite README"
check_file "TESTING_IMPLEMENTATION.md" "Implementation documentation"
check_file "TESTING_QUICK_START.md" "Quick start guide"
check_file "TEST_IMPLEMENTATION_COMPLETE.md" "Completion summary"
echo ""

# Count test cases in files
echo "📊 Analyzing Test Coverage..."

if command -v rg &> /dev/null; then
    echo "Counting test cases..."
    API_TESTS=$(rg -c "^\s*#\[tokio::test\]|^\s*#\[test\]" tests/integration/api_tests.rs 2>/dev/null || echo "0")
    CONV_TESTS=$(rg -c "^\s*#\[tokio::test\]|^\s*#\[test\]" tests/integration/conversation_tests.rs 2>/dev/null || echo "0")
    WORKFLOW_TESTS=$(rg -c "^\s*#\[tokio::test\]|^\s*#\[test\]" tests/integration/workflow_tests.rs 2>/dev/null || echo "0")
    COMMON_TESTS=$(rg -c "^\s*#\[tokio::test\]|^\s*#\[test\]" tests/common/*.rs 2>/dev/null | awk '{sum+=$1} END {print sum}' || echo "0")

    echo "  - API Tests: $API_TESTS"
    echo "  - Conversation Tests: $CONV_TESTS"
    echo "  - Workflow Tests: $WORKFLOW_TESTS"
    echo "  - Common Module Tests: $COMMON_TESTS"
    TOTAL=$((API_TESTS + CONV_TESTS + WORKFLOW_TESTS + COMMON_TESTS))
    echo "  - Total: $TOTAL test cases"
else
    echo "  ${YELLOW}Note: Install ripgrep (rg) for test counting${NC}"
fi
echo ""

# Check file sizes
echo "💾 File Sizes..."
echo "Integration Tests:"
ls -lh tests/integration/*.rs 2>/dev/null | awk '{printf "  %s: %s\n", $9, $5}' | sed 's|tests/integration/||'
echo "Common Utilities:"
ls -lh tests/common/*.rs 2>/dev/null | awk '{printf "  %s: %s\n", $9, $5}' | sed 's|tests/common/||'
echo "Benchmarks:"
ls -lh benches/*.rs 2>/dev/null | awk '{printf "  %s: %s\n", $9, $5}' | sed 's|benches/||'
echo ""

# Check Makefile targets
echo "🎯 Checking Makefile Targets..."
if grep -q "^test-unit:" Makefile 2>/dev/null; then
    echo -e "${GREEN}✓${NC} test-unit target"
    ((PASSED++))
else
    echo -e "${RED}✗${NC} test-unit target"
    ((FAILED++))
fi

if grep -q "^test-integration:" Makefile 2>/dev/null; then
    echo -e "${GREEN}✓${NC} test-integration target"
    ((PASSED++))
else
    echo -e "${RED}✗${NC} test-integration target"
    ((FAILED++))
fi

if grep -q "^bench:" Makefile 2>/dev/null; then
    echo -e "${GREEN}✓${NC} bench target"
    ((PASSED++))
else
    echo -e "${RED}✗${NC} bench target"
    ((FAILED++))
fi

if grep -q "^coverage:" Makefile 2>/dev/null; then
    echo -e "${GREEN}✓${NC} coverage target"
    ((PASSED++))
else
    echo -e "${RED}✗${NC} coverage target"
    ((FAILED++))
fi
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📈 Verification Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All checks passed!${NC}"
    echo ""
    echo "Next Steps:"
    echo "  1. Run tests: make test-unit"
    echo "  2. Run benchmarks: make bench"
    echo "  3. Check coverage: make coverage"
    echo "  4. Read quick start: cat TESTING_QUICK_START.md"
    exit 0
else
    echo -e "${RED}❌ Some checks failed. Please review the output above.${NC}"
    exit 1
fi

#!/bin/bash

# Health Check Endpoint - PR Validation Script
# This script validates that the health check endpoint is properly implemented

set -e

echo "═══════════════════════════════════════════════════════════════"
echo "   Health Check Endpoint - PR Validation Script"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

FAILED=0
PASSED=0

# Function to print test results
test_result() {
  local test_name=$1
  local result=$2
  
  if [ $result -eq 0 ]; then
    echo -e "${GREEN}✓ PASS${NC}: $test_name"
    ((PASSED++))
  else
    echo -e "${RED}✗ FAIL${NC}: $test_name"
    ((FAILED++))
  fi
}

# Test 1: Verify app.service.ts has getHealth method
echo ""
echo "Test 1: Checking AppService implementation..."
if grep -q "getHealth()" backend/src/app.service.ts; then
  test_result "AppService.getHealth() method exists" 0
else
  test_result "AppService.getHealth() method exists" 1
fi

# Test 2: Verify app.controller.ts has health endpoint
echo ""
echo "Test 2: Checking AppController implementation..."
if grep -q "@Get('health')" backend/src/app.controller.ts; then
  test_result "AppController @Get('health') decorator exists" 0
else
  test_result "AppController @Get('health') decorator exists" 1
fi

# Test 3: Verify response has status field
echo ""
echo "Test 3: Checking response structure..."
if grep -q "status: 'ok'" backend/src/app.service.ts; then
  test_result "Response includes status: 'ok'" 0
else
  test_result "Response includes status: 'ok'" 1
fi

# Test 4: Verify response has timestamp field
echo ""
echo "Test 4: Checking timestamp field..."
if grep -q "toISOString()" backend/src/app.service.ts; then
  test_result "Response includes ISO timestamp" 0
else
  test_result "Response includes ISO timestamp" 1
fi

# Test 5: Verify unit test exists
echo ""
echo "Test 5: Checking unit tests..."
if grep -q "should return status ok with timestamp" backend/src/app.controller.spec.ts; then
  test_result "Unit test for health endpoint exists" 0
else
  test_result "Unit test for health endpoint exists" 1
fi

# Test 6: Verify no auth decorator on health endpoint
echo ""
echo "Test 6: Checking authentication requirements..."
grep -A 2 "@Get('health')" backend/src/app.controller.ts > /tmp/health_endpoint.txt
if ! grep -q "UseGuards\|Auth\|ApiKey" /tmp/health_endpoint.txt; then
  test_result "Health endpoint has no auth requirements" 0
else
  test_result "Health endpoint has no auth requirements" 1
fi

# Test 7: Check file structure
echo ""
echo "Test 7: Verifying modified files exist..."
FILES_OK=0
[ -f backend/src/app.service.ts ] && ((FILES_OK++)) || ((FILES_OK--))
[ -f backend/src/app.controller.ts ] && ((FILES_OK++)) || ((FILES_OK--))
[ -f backend/src/app.controller.spec.ts ] && ((FILES_OK++)) || ((FILES_OK--))

if [ $FILES_OK -eq 3 ]; then
  test_result "All required files exist" 0
else
  test_result "All required files exist" 1
fi

# Summary
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "                         TEST SUMMARY"
echo "═══════════════════════════════════════════════════════════════"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
  echo -e "${GREEN}✓ All validation tests PASSED!${NC}"
  echo ""
  echo "Next steps:"
  echo "1. cd backend"
  echo "2. npm install"
  echo "3. npm test -- app.controller.spec.ts"
  echo "4. npm run start:dev"
  echo "5. curl http://localhost:3000/health"
  echo ""
  exit 0
else
  echo -e "${RED}✗ Some validation tests FAILED!${NC}"
  echo ""
  exit 1
fi

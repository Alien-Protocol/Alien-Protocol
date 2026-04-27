#!/bin/bash

# Simple PR Validation Script

echo "════════════════════════════════════════════════════════════"
echo "  Health Check Endpoint - PR Validation"
echo "════════════════════════════════════════════════════════════"
echo ""

PASS=0
FAIL=0

# Test 1
echo -n "✓ AppService has getHealth() method... "
if grep -q "getHealth()" backend/src/app.service.ts; then
  echo "PASS"; ((PASS++))
else
  echo "FAIL"; ((FAIL++))
fi

# Test 2
echo -n "✓ AppController has health endpoint... "
if grep -q "getHealth()" backend/src/app.controller.ts; then
  echo "PASS"; ((PASS++))
else
  echo "FAIL"; ((FAIL++))
fi

# Test 3
echo -n "✓ Response has status ok field... "
if grep -q "status: 'ok'" backend/src/app.service.ts; then
  echo "PASS"; ((PASS++))
else
  echo "FAIL"; ((FAIL++))
fi

# Test 4
echo -n "✓ Response has timestamp field... "
if grep -q "toISOString()" backend/src/app.service.ts; then
  echo "PASS"; ((PASS++))
else
  echo "FAIL"; ((FAIL++))
fi

# Test 5
echo -n "✓ Unit test exists... "
if grep -q "should return status ok with timestamp" backend/src/app.controller.spec.ts; then
  echo "PASS"; ((PASS++))
else
  echo "FAIL"; ((FAIL++))
fi

# Test 6
echo -n "✓ Test validates status field... "
if grep -q "expect(result.status).toBe('ok')" backend/src/app.controller.spec.ts; then
  echo "PASS"; ((PASS++))
else
  echo "FAIL"; ((FAIL++))
fi

# Test 7
echo -n "✓ Test validates timestamp field... "
if grep -q "expect(result.timestamp).toBeDefined()" backend/src/app.controller.spec.ts; then
  echo "PASS"; ((PASS++))
else
  echo "FAIL"; ((FAIL++))
fi

echo ""
echo "════════════════════════════════════════════════════════════"
echo "RESULTS: $PASS PASSED | $FAIL FAILED"
echo "════════════════════════════════════════════════════════════"

if [ $FAIL -eq 0 ]; then
  echo "✓ All checks PASSED - Ready for testing!"
  exit 0
else
  echo "✗ Some checks FAILED"
  exit 1
fi

9- ## 🎯 Description
Add a simple GET `/health` endpoint that returns the service status and current UTC timestamp. This enables load balancers and monitoring tools to verify the backend is running.

## 📋 Type of Change
- [x] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update

## ✨ Changes Made

### Backend Controller (`backend/src/app.controller.ts`)
- Added `@Get('health')` endpoint
- Returns health status with UTC timestamp
- No authentication required

### Backend Service (`backend/src/app.service.ts`)
- Added `getHealth()` method
- Returns object with `status: 'ok'` and current `timestamp` in ISO format

### Unit Tests (`backend/src/app.controller.spec.ts`)
- Added health endpoint test suite
- Validates response structure (status and timestamp)
- Verifies timestamp is valid ISO format

## ✅ Acceptance Criteria

- [x] GET `/health` returns HTTP 200
- [x] Response contains `status: 'ok'` and valid UTC `timestamp`
- [x] Endpoint requires no authentication
- [x] Unit tests pass
- [x] Code follows project conventions

## 🧪 Testing Instructions

### Unit Tests
```bash
cd backend
npm test -- app.controller.spec.ts
```

### Integration Test
```bash
cd backend
npm run start:dev
# In another terminal:
curl http://localhost:3000/health
```

### Expected Response
```json
{
  "status": "ok",
  "timestamp": "2026-04-27T14:23:45.123Z"
}
```

## 📝 Related Issues
- Closes: #(issue number if applicable)

## 🔍 Checklist

- [x] Code follows project style guidelines
- [x] I have performed a self-review of my own code
- [x] Unit tests pass locally
- [x] No breaking changes introduced
- [x] Documentation updated (if needed)
- [x] Commit message follows conventional commits format

## 🏆 Priority
**LOW** - Non-critical enhancement

## ☕ Difficulty
**one-coffee** - Simple implementation

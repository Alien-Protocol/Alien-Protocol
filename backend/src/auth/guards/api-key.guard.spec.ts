import { ExecutionContext, UnauthorizedException } from '@nestjs/common';
import { ApiKeyGuard } from './api-key.guard';

describe('ApiKeyGuard', () => {
  let guard: ApiKeyGuard;

  beforeEach(() => {
    guard = new ApiKeyGuard();
    process.env.API_KEYS = 'test-key-1,test-key-2';
  });

  const createMockContext = (headers: any = {}, method: string = 'POST', url: string = '/admin') => {
    return {
      switchToHttp: () => ({
        getRequest: () => ({
          headers,
          method,
          url,
        }),
      }),
    } as unknown as ExecutionContext;
  };

  it('should be defined', () => {
    expect(guard).toBeDefined();
  });

  it('should allow public read endpoints without API key', () => {
    const context = createMockContext({}, 'GET', '/vault/123/balance');
    expect(guard.canActivate(context)).toBe(true);
  });

  it('should allow valid API keys', () => {
    const context = createMockContext({ 'x-api-key': 'test-key-1' });
    expect(guard.canActivate(context)).toBe(true);
  });

  it('should throw UnauthorizedException if API key is missing', () => {
    const context = createMockContext({});
    expect(() => guard.canActivate(context)).toThrow(UnauthorizedException);
    expect(() => guard.canActivate(context)).toThrow('API key is missing');
  });

  it('should throw UnauthorizedException if API key is invalid', () => {
    const context = createMockContext({ 'x-api-key': 'invalid-key' });
    expect(() => guard.canActivate(context)).toThrow(UnauthorizedException);
    expect(() => guard.canActivate(context)).toThrow('Invalid API key');
  });

  it('should support multiple API keys from env var', () => {
    const context = createMockContext({ 'x-api-key': 'test-key-2' });
    expect(guard.canActivate(context)).toBe(true);
  });
});

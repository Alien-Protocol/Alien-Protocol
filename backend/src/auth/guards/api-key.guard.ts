import { CanActivate, ExecutionContext, Injectable, UnauthorizedException } from '@nestjs/common';

@Injectable()
export class ApiKeyGuard implements CanActivate {
  canActivate(context: ExecutionContext): boolean {
    const request = context.switchToHttp().getRequest();
    const { method, url } = request;

    // Public read endpoints: GET /resolve/*, GET /vault/*, GET /auction/*
    const publicPrefixes = ['/resolve', '/vault', '/auction'];
    const isPublic = method === 'GET' && publicPrefixes.some((prefix) => url.startsWith(prefix));

    if (isPublic) {
      return true;
    }

    const apiKey = request.headers['x-api-key'];

    if (!apiKey) {
      throw new UnauthorizedException('API key is missing');
    }

    const validApiKeys = (process.env.API_KEYS || '').split(',').map((key) => key.trim());
    
    if (!validApiKeys.includes(apiKey)) {
      throw new UnauthorizedException('Invalid API key');
    }

    return true;
  }
}

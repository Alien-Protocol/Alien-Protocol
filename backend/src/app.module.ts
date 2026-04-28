import { Module } from '@nestjs/common';
import { LoggerModule } from 'nestjs-pino';
import { ResolverModule } from './resolver/resolver.module';
import { VaultModule } from './vault/vault.module';
import { AuctionModule } from './auction/auction.module';
import { HealthModule } from './health/health.module';

@Module({
  imports: [
    LoggerModule.forRoot({
      pinoHttp: {
        level: process.env.LOG_LEVEL || 'info',
        transport: process.env.NODE_ENV !== 'production'
          ? {
              target: 'pino-pretty',
              options: {
                singleLine: true,
                colorize: true,
              },
            }
          : undefined,
        serializers: {
          req: () => ({
            method: 'REQ',
            url: 'URL',
            headers: 'HEADERS',
          }),
        },
      },
    }),
    ResolverModule,
    VaultModule,
    AuctionModule,
    HealthModule,
  ],
})
export class AppModule {}

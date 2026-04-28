import { Module } from '@nestjs/common';
import { TerminusModule } from '@nestjs/terminus';
import { HealthController } from './health.controller';
import { StellarHealthIndicator } from './stellar-health.indicator';
import { ConfigModule } from '../config/config.module';
import { DatabaseModule } from '../database/database.module';

@Module({
  imports: [TerminusModule, ConfigModule, DatabaseModule],
  controllers: [HealthController],
  providers: [StellarHealthIndicator],
})
export class HealthModule {}

import { Controller, Get } from '@nestjs/common';
import { HealthCheck, HealthCheckService, HttpHealthIndicator, TypeOrmHealthIndicator } from '@nestjs/terminus';
import { StellarHealthIndicator } from './stellar-health.indicator';

@Controller('health')
export class HealthController {
  constructor(
    private health: HealthCheckService,
    private http: HttpHealthIndicator,
    private stellar: StellarHealthIndicator,
    private db: TypeOrmHealthIndicator,
  ) {}

  @Get()
  @HealthCheck()
  check() {
    const port = process.env.PORT || '3000';
    return this.health.check([
      () => this.http.pingCheck('http-server', `http://localhost:${port}`),
      () => this.stellar.pingCheck('stellar-rpc'),
      () => this.db.pingCheck('database'),
    ]);
  }
}

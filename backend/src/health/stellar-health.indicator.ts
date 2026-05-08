import { Injectable } from '@nestjs/common';
import { HealthIndicator, HealthIndicatorResult } from '@nestjs/terminus';
import { ConfigService } from '../config/config.service';

@Injectable()
export class StellarHealthIndicator extends HealthIndicator {
  constructor(private configService: ConfigService) {
    super();
  }

  async pingCheck(key: string): Promise<HealthIndicatorResult> {
    const stellarRpcUrl = this.configService.stellarRpcUrl;
    
    try {
      const response = await fetch(stellarRpcUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'getHealth',
        }),
      });

      if (response.ok) {
        return this.getStatus(key, true);
      } else {
        throw new Error(`Stellar RPC returned status ${response.status}`);
      }
    } catch (error) {
      return this.getStatus(key, false, { message: error.message });
    }
  }
}

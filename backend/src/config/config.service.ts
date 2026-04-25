import { Injectable } from '@nestjs/common';
import { ConfigService as NestConfigService } from '@nestjs/config';

@Injectable()
export class ConfigService {
  constructor(private configService: NestConfigService) {}

  get stellarRpcUrl(): string {
    return this.configService.get<string>('STELLAR_RPC_URL') || 'https://soroban-testnet.stellar.org';
  }

  get coreContractId(): string {
    return this.configService.get<string>('CORE_CONTRACT_ID') || '';
  }

  get escrowContractId(): string {
    return this.configService.get<string>('ESCROW_CONTRACT_ID') || '';
  }

  get factoryContractId(): string {
    return this.configService.get<string>('FACTORY_CONTRACT_ID') || '';
  }

  get auctionContractId(): string {
    return this.configService.get<string>('AUCTION_CONTRACT_ID') || '';
  }

  get databaseUrl(): string {
    return this.configService.get<string>('DATABASE_URL') || '';
  }
}

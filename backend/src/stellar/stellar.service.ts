import { Injectable, OnModuleInit, Logger } from '@nestjs/common';
import { rpc, Contract, Address, xdr } from '@stellar/stellar-sdk';
import { ConfigService } from '../config/config.service';
import { StellarRpcException } from './stellar.exceptions';
import {
  ChainType,
  GetOwnerResult,
  ResolveStellarResult,
  GetChainAddressResult,
  GetVaultBalanceResult,
  GetScheduledPaymentResult,
  IsVaultActiveResult,
  GetCreatedAtResult,
  ScheduledPayment,
} from './stellar.types';

@Injectable()
export class StellarService implements OnModuleInit {
  private readonly logger = new Logger(StellarService.name);
  private server: rpc.Server;

  constructor(private configService: ConfigService) {
    this.server = new rpc.Server(this.configService.stellarRpcUrl);
  }

  async onModuleInit() {
    try {
      // Test the connection by getting network info
      const network = await this.server.getNetwork();
      this.logger.log(`Connected to Stellar network: ${network.passphrase} at ${this.configService.stellarRpcUrl}`);
    } catch (error) {
      this.logger.error(`Failed to connect to Stellar RPC: ${error.message}`);
    }
  }

  getServer(): rpc.Server {
    return this.server;
  }

  getCoreContract(): Contract {
    return new Contract(this.configService.coreContractId);
  }

  getEscrowContract(): Contract {
    return new Contract(this.configService.escrowContractId);
  }

  getFactoryContract(): Contract {
    return new Contract(this.configService.factoryContractId);
  }

  getAuctionContract(): Contract {
    return new Contract(this.configService.auctionContractId);
  }

  /**
    * Retrieves a paginated list of auctions from the on-chain contract.
    * Supports optional status filtering.
    *
    * @param page - Page number (1-indexed)
    * @param limit - Number of auctions per page (max 100)
    * @param status - Optional status filter ('active', 'closed', 'claimed')
    * @returns A promise that resolves to an array of auction data.
    */
  async getAuctions(page: number = 1, limit: number = 20, status?: string): Promise<any[]> {
    // Placeholder: actual implementation would call the auction contract's list_auctions method
    // For now, return mock data to satisfy the interface
    const mockAuctions = [
      { id: 1, username: 'satoshi', highestBid: '150.00', endTime: 1745000000, status: 'open' },
      { id: 2, username: 'vitalik', highestBid: '200.00', endTime: 1746000000, status: 'closed' },
      { id: 3, username: 'alice', highestBid: '75.00', endTime: 1747000000, status: 'open' },
    ];

    // Apply status filter if provided
    let filtered = mockAuctions;
    if (status) {
      filtered = mockAuctions.filter((a) => a.status === status);
    }

    // Apply pagination
    const start = (page - 1) * limit;
    const end = start + limit;
    return filtered.slice(start, end);
  }
}

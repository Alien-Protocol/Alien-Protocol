import { Injectable, OnModuleInit, Logger } from '@nestjs/common';
import { rpc, Contract } from '@stellar/stellar-sdk';
import { ConfigService } from '../config/config.service';

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
}

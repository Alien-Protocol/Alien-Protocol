import { OnModuleInit } from '@nestjs/common';
import { rpc, Contract } from '@stellar/stellar-sdk';
import { ConfigService } from '../config/config.service';
export declare class StellarService implements OnModuleInit {
    private configService;
    private readonly logger;
    private server;
    constructor(configService: ConfigService);
    onModuleInit(): Promise<void>;
    getServer(): rpc.Server;
    getCoreContract(): Contract;
    getEscrowContract(): Contract;
    getFactoryContract(): Contract;
    getAuctionContract(): Contract;
}

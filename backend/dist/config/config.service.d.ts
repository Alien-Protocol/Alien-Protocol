import { ConfigService as NestConfigService } from '@nestjs/config';
export declare class ConfigService {
    private configService;
    constructor(configService: NestConfigService);
    get stellarRpcUrl(): string;
    get coreContractId(): string;
    get escrowContractId(): string;
    get factoryContractId(): string;
    get auctionContractId(): string;
    get databaseUrl(): string;
}

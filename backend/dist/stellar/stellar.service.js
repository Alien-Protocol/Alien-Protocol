"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
var __metadata = (this && this.__metadata) || function (k, v) {
    if (typeof Reflect === "object" && typeof Reflect.metadata === "function") return Reflect.metadata(k, v);
};
var StellarService_1;
Object.defineProperty(exports, "__esModule", { value: true });
exports.StellarService = void 0;
const common_1 = require("@nestjs/common");
const stellar_sdk_1 = require("@stellar/stellar-sdk");
const config_service_1 = require("../config/config.service");
let StellarService = StellarService_1 = class StellarService {
    constructor(configService) {
        this.configService = configService;
        this.logger = new common_1.Logger(StellarService_1.name);
        this.server = new stellar_sdk_1.rpc.Server(this.configService.stellarRpcUrl);
    }
    async onModuleInit() {
        try {
            const network = await this.server.getNetwork();
            this.logger.log(`Connected to Stellar network: ${network.passphrase} at ${this.configService.stellarRpcUrl}`);
        }
        catch (error) {
            this.logger.error(`Failed to connect to Stellar RPC: ${error.message}`);
        }
    }
    getServer() {
        return this.server;
    }
    getCoreContract() {
        return new stellar_sdk_1.Contract(this.configService.coreContractId);
    }
    getEscrowContract() {
        return new stellar_sdk_1.Contract(this.configService.escrowContractId);
    }
    getFactoryContract() {
        return new stellar_sdk_1.Contract(this.configService.factoryContractId);
    }
    getAuctionContract() {
        return new stellar_sdk_1.Contract(this.configService.auctionContractId);
    }
};
exports.StellarService = StellarService;
exports.StellarService = StellarService = StellarService_1 = __decorate([
    (0, common_1.Injectable)(),
    __metadata("design:paramtypes", [config_service_1.ConfigService])
], StellarService);
//# sourceMappingURL=stellar.service.js.map
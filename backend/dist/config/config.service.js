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
var _a;
Object.defineProperty(exports, "__esModule", { value: true });
exports.ConfigService = void 0;
const common_1 = require("@nestjs/common");
const config_1 = require("@nestjs/config");
let ConfigService = class ConfigService {
    constructor(configService) {
        this.configService = configService;
    }
    get stellarRpcUrl() {
        return this.configService.get('STELLAR_RPC_URL') || 'https://soroban-testnet.stellar.org';
    }
    get coreContractId() {
        return this.configService.get('CORE_CONTRACT_ID') || '';
    }
    get escrowContractId() {
        return this.configService.get('ESCROW_CONTRACT_ID') || '';
    }
    get factoryContractId() {
        return this.configService.get('FACTORY_CONTRACT_ID') || '';
    }
    get auctionContractId() {
        return this.configService.get('AUCTION_CONTRACT_ID') || '';
    }
    get databaseUrl() {
        return this.configService.get('DATABASE_URL') || '';
    }
};
exports.ConfigService = ConfigService;
exports.ConfigService = ConfigService = __decorate([
    (0, common_1.Injectable)(),
    __metadata("design:paramtypes", [typeof (_a = typeof config_1.ConfigService !== "undefined" && config_1.ConfigService) === "function" ? _a : Object])
], ConfigService);
//# sourceMappingURL=config.service.js.map
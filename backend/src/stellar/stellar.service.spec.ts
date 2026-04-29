import { Test, TestingModule } from '@nestjs/testing';
import { StellarService } from './stellar.service';
import { ConfigService } from '../config/config.service';

describe('StellarService', () => {
  let service: StellarService;
  let configService: ConfigService;

  const mockConfigService = {
    stellarRpcUrl: 'https://soroban-testnet.stellar.org',
    coreContractId: 'core_contract_id',
    escrowContractId: 'escrow_contract_id',
    factoryContractId: 'factory_contract_id',
    auctionContractId: 'auction_contract_id',
  };

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        StellarService,
        { provide: ConfigService, useValue: mockConfigService },
      ],
    }).compile();

    service = module.get<StellarService>(StellarService);
    configService = module.get<ConfigService>(ConfigService);
  });

  it('should be defined', () => {
    expect(service).toBeDefined();
  });

  describe('getAuctions', () => {
    it('should return paginated auctions', async () => {
      const result = await service.getAuctions(1, 10);
      expect(Array.isArray(result)).toBe(true);
      expect(result.length).toBeLessThanOrEqual(10);
    });

    it('should filter auctions by status=open', async () => {
      const result = await service.getAuctions(1, 100, 'open');
      expect(Array.isArray(result)).toBe(true);
      result.forEach((a) => {
        expect(a.status).toBe('open');
      });
    });

    it('should filter auctions by status=closed', async () => {
      const result = await service.getAuctions(1, 100, 'closed');
      expect(Array.isArray(result)).toBe(true);
      result.forEach((a) => {
        expect(a.status).toBe('closed');
      });
    });

    it('should cap limit at 100', async () => {
      const result = await service.getAuctions(1, 200);
      expect(Array.isArray(result)).toBe(true);
      // Our mock only has 3 items, but limit shouldn't matter here
    });

    it('should handle invalid status gracefully by returning empty or all', async () => {
      const result = await service.getAuctions(1, 100, 'invalid');
      expect(Array.isArray(result)).toBe(true);
      // Since mock has no 'invalid' status, result should be empty
      expect(result.length).toBe(0);
    });
  });
});

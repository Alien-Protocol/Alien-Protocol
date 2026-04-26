import { Test, TestingModule } from '@nestjs/testing';
import { VaultService } from './vault.service';
import { PrismaService } from '../prisma.service';
import { SorobanService } from '../soroban.service';
import { NotFoundException } from '@nestjs/common';

describe('VaultService', () => {
  let service: VaultService;
  let prisma: PrismaService;
  let soroban: SorobanService;

  const mockPrisma = {
    vault: {
      findUnique: jest.fn(),
    },
    scheduledPayment: {
      findUnique: jest.fn(),
      findMany: jest.fn(),
    },
    autoPayRule: {
      findMany: jest.fn(),
    },
  };

  const mockSoroban = {
    getVaultBalance: jest.fn(),
    getScheduledPayment: jest.fn(),
    isVaultActive: jest.fn(),
  };

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        VaultService,
        { provide: PrismaService, useValue: mockPrisma },
        { provide: SorobanService, useValue: mockSoroban },
      ],
    }).compile();

    service = module.get<VaultService>(VaultService);
    prisma = module.get<PrismaService>(PrismaService);
    soroban = module.get<SorobanService>(SorobanService);
  });

  it('should be defined', () => {
    expect(service).toBeDefined();
  });

  describe('getBalance', () => {
    it('should return balance from DB if found', async () => {
      const mockVault = { commitment: '0x123', balance: '1000', isActive: true };
      (prisma.vault.findUnique as jest.Mock).mockResolvedValue(mockVault);

      const result = await service.getBalance('0x123');
      expect(result).toEqual({ balance: '1000', isActive: true });
      expect(prisma.vault.findUnique).toHaveBeenCalledWith({ where: { commitment: '0x123' } });
    });

    it('should fallback to Soroban if not in DB', async () => {
      (prisma.vault.findUnique as jest.Mock).mockResolvedValue(null);
      (soroban.getVaultBalance as jest.Mock).mockResolvedValue(BigInt(500));

      const result = await service.getBalance('0x123');
      expect(result).toEqual({ balance: '500', isActive: true });
      expect(soroban.getVaultBalance).toHaveBeenCalledWith('0x123');
    });

    it('should throw NotFoundException if not in DB and not in contract', async () => {
      (prisma.vault.findUnique as jest.Mock).mockResolvedValue(null);
      (soroban.getVaultBalance as jest.Mock).mockResolvedValue(null);

      await expect(service.getBalance('0x123')).rejects.toThrow(NotFoundException);
    });
  });

  describe('getPaymentById', () => {
    it('should return payment from DB if found', async () => {
      const mockPayment = { id: 1, amount: '100', releaseAt: BigInt(12345) };
      (prisma.scheduledPayment.findUnique as jest.Mock).mockResolvedValue(mockPayment);

      const result = await service.getPaymentById(1);
      expect(result.amount).toBe('100');
      expect(result.releaseAt).toBe('12345');
    });

    it('should fallback to Soroban if not in DB', async () => {
      (prisma.scheduledPayment.findUnique as jest.Mock).mockResolvedValue(null);
      const mockContractPayment = { id: 1, amount: '200', releaseAt: '67890' };
      (soroban.getScheduledPayment as jest.Mock).mockResolvedValue(mockContractPayment);

      const result = await service.getPaymentById(1);
      expect(result).toEqual(mockContractPayment);
    });
  });
});

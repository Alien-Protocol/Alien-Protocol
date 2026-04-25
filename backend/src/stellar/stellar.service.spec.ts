import { Test, TestingModule } from '@nestjs/testing';
import { Logger } from '@nestjs/common';
import { rpc, Contract, Address, xdr } from '@stellar/stellar-sdk';
import { StellarService } from './stellar.service';
import { ConfigService } from '../config/config.service';
import { StellarRpcException } from './stellar.exceptions';
import { ChainType } from './stellar.types';

describe('StellarService', () => {
  let service: StellarService;
  let configService: ConfigService;
  let mockServer: jest.Mocked<rpc.Server>;
  let mockContract: jest.Mocked<Contract>;

  const mockConfig = {
    stellarRpcUrl: 'https://test-rpc.stellar.org',
    coreContractId: 'CORE_CONTRACT_ID',
    escrowContractId: 'ESCROW_CONTRACT_ID',
    factoryContractId: 'FACTORY_CONTRACT_ID',
    auctionContractId: 'AUCTION_CONTRACT_ID',
  };

  beforeEach(async () => {
    // Mock the server
    mockServer = {
      getNetwork: jest.fn(),
      simulateTransaction: jest.fn(),
    } as any;

    // Mock the contract
    mockContract = {
      call: jest.fn(),
    } as any;

    const module: TestingModule = await Test.createTestingModule({
      providers: [
        StellarService,
        {
          provide: ConfigService,
          useValue: mockConfig,
        },
      ],
    }).compile();

    service = module.get<StellarService>(StellarService);
    configService = module.get<ConfigService>(ConfigService);

    // Replace the server instance with our mock
    (service as any).server = mockServer;

    // Mock Contract constructor
    jest.spyOn(Contract.prototype, 'call').mockImplementation(mockContract.call);
  });

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe('onModuleInit', () => {
    it('should connect to Stellar network successfully', async () => {
      mockServer.getNetwork.mockResolvedValue({
        passphrase: 'Test SDF Network ; September 2015',
      } as any);

      const loggerSpy = jest.spyOn(Logger.prototype, 'log');

      await service.onModuleInit();

      expect(mockServer.getNetwork).toHaveBeenCalled();
      expect(loggerSpy).toHaveBeenCalledWith(
        expect.stringContaining('Connected to Stellar network'),
      );
    });

    it('should handle connection errors gracefully', async () => {
      mockServer.getNetwork.mockRejectedValue(new Error('Connection failed'));

      const loggerSpy = jest.spyOn(Logger.prototype, 'error');

      await service.onModuleInit();

      expect(loggerSpy).toHaveBeenCalledWith(
        expect.stringContaining('Failed to connect to Stellar RPC'),
      );
    });
  });

  describe('getOwner', () => {
    const commitment = 'test_commitment';

    it('should return owner address when found', async () => {
      const ownerAddress = 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX';
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      // Mock XDR parsing
      const mockScVal = {
        switch: jest.fn().mockReturnValue('not_void'),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);
      jest.spyOn(Address, 'fromScVal').mockReturnValue({ toString: () => ownerAddress } as any);

      const result = await service.getOwner(commitment);

      expect(result).toBe(ownerAddress);
      expect(mockContract.call).toHaveBeenCalledWith('get_owner', commitment);
    });

    it('should return null when owner not found', async () => {
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      // Mock XDR parsing for void result
      const mockScVal = {
        switch: jest.fn().mockReturnValue(xdr.ScValType.scvVoid()),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);
      jest.spyOn(xdr.ScValType, 'scvVoid').mockReturnValue('void_type' as any);

      const result = await service.getOwner(commitment);

      expect(result).toBeNull();
    });

    it('should throw StellarRpcException on RPC error', async () => {
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        error: 'Contract error',
      } as any);

      await expect(service.getOwner(commitment)).rejects.toThrow(StellarRpcException);
    });
  });

  describe('resolveUsername', () => {
    const usernameHash = 'test_username_hash';

    it('should return stellar address for username hash', async () => {
      const stellarAddress = 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX';
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {};
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);
      jest.spyOn(Address, 'fromScVal').mockReturnValue({ toString: () => stellarAddress } as any);

      const result = await service.resolveUsername(usernameHash);

      expect(result).toBe(stellarAddress);
      expect(mockContract.call).toHaveBeenCalledWith('resolve_stellar', usernameHash);
    });

    it('should throw StellarRpcException when no return value', async () => {
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: {},
      } as any);

      await expect(service.resolveUsername(usernameHash)).rejects.toThrow(StellarRpcException);
    });
  });

  describe('getChainAddress', () => {
    const usernameHash = 'test_username_hash';
    const chain = ChainType.Evm;

    it('should return chain address when found', async () => {
      const chainAddress = '0x1234567890123456789012345678901234567890';
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {
        switch: jest.fn().mockReturnValue('not_void'),
        bytes: jest.fn().mockReturnValue(Buffer.from(chainAddress, 'utf8')),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);

      const result = await service.getChainAddress(usernameHash, chain);

      expect(result).toBe(chainAddress);
      expect(mockContract.call).toHaveBeenCalledWith('get_chain_address', usernameHash, chain);
    });

    it('should return null when address not found', async () => {
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {
        switch: jest.fn().mockReturnValue(xdr.ScValType.scvVoid()),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);
      jest.spyOn(xdr.ScValType, 'scvVoid').mockReturnValue('void_type' as any);

      const result = await service.getChainAddress(usernameHash, chain);

      expect(result).toBeNull();
    });
  });

  describe('getVaultBalance', () => {
    const commitment = 'test_commitment';

    it('should return vault balance when found', async () => {
      const balance = '1000000000';
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {
        switch: jest.fn().mockReturnValue('not_void'),
        i128: jest.fn().mockReturnValue({ toString: () => balance }),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);

      const result = await service.getVaultBalance(commitment);

      expect(result).toBe(balance);
      expect(mockContract.call).toHaveBeenCalledWith('get_balance', commitment);
    });

    it('should return null when balance not found', async () => {
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {
        switch: jest.fn().mockReturnValue(xdr.ScValType.scvVoid()),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);
      jest.spyOn(xdr.ScValType, 'scvVoid').mockReturnValue('void_type' as any);

      const result = await service.getVaultBalance(commitment);

      expect(result).toBeNull();
    });
  });

  describe('getScheduledPayment', () => {
    const paymentId = 123;

    it('should return scheduled payment when found', async () => {
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockFields = [
        { val: () => ({ bytes: () => Buffer.from('from_address', 'hex') }) },
        { val: () => ({ bytes: () => Buffer.from('to_address', 'hex') }) },
        { val: () => ({}) }, // token address
        { val: () => ({ i128: () => ({ toString: () => '1000000' }) }) },
        { val: () => ({ u64: () => 1640995200 }) },
        { val: () => ({ b: () => false }) },
      ];

      const mockScVal = {
        switch: jest.fn().mockReturnValue('not_void'),
        instance: jest.fn().mockReturnValue({
          instanceValue: jest.fn().mockReturnValue({
            map: jest.fn().mockReturnValue(mockFields),
          }),
        }),
      };

      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);
      jest.spyOn(Address, 'fromScVal').mockReturnValue({ toString: () => 'GTOKEN' } as any);

      const result = await service.getScheduledPayment(paymentId);

      expect(result).toEqual({
        from: expect.any(String),
        to: expect.any(String),
        token: 'GTOKEN',
        amount: '1000000',
        release_at: 1640995200,
        executed: false,
      });
    });

    it('should return null when payment not found', async () => {
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {
        switch: jest.fn().mockReturnValue(xdr.ScValType.scvVoid()),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);
      jest.spyOn(xdr.ScValType, 'scvVoid').mockReturnValue('void_type' as any);

      const result = await service.getScheduledPayment(paymentId);

      expect(result).toBeNull();
    });
  });

  describe('isVaultActive', () => {
    const commitment = 'test_commitment';

    it('should return true when vault is active', async () => {
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {
        switch: jest.fn().mockReturnValue('not_void'),
        b: jest.fn().mockReturnValue(true),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);

      const result = await service.isVaultActive(commitment);

      expect(result).toBe(true);
      expect(mockContract.call).toHaveBeenCalledWith('is_vault_active', commitment);
    });

    it('should return false when vault is inactive', async () => {
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {
        switch: jest.fn().mockReturnValue('not_void'),
        b: jest.fn().mockReturnValue(false),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);

      const result = await service.isVaultActive(commitment);

      expect(result).toBe(false);
    });
  });

  describe('getCreatedAt', () => {
    const commitment = 'test_commitment';

    it('should return creation timestamp when found', async () => {
      const timestamp = 1640995200;
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {
        switch: jest.fn().mockReturnValue('not_void'),
        u64: jest.fn().mockReturnValue(timestamp),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);

      const result = await service.getCreatedAt(commitment);

      expect(result).toBe(timestamp);
      expect(mockContract.call).toHaveBeenCalledWith('get_created_at', commitment);
    });

    it('should return null when timestamp not found', async () => {
      const mockRetval = 'mock_xdr_data';
      
      mockContract.call.mockReturnValue('mock_transaction' as any);
      mockServer.simulateTransaction.mockResolvedValue({
        result: { retval: mockRetval },
      } as any);

      const mockScVal = {
        switch: jest.fn().mockReturnValue(xdr.ScValType.scvVoid()),
      };
      jest.spyOn(xdr.ScVal, 'fromXDR').mockReturnValue(mockScVal as any);
      jest.spyOn(xdr.ScValType, 'scvVoid').mockReturnValue('void_type' as any);

      const result = await service.getCreatedAt(commitment);

      expect(result).toBeNull();
    });
  });

  describe('contract getters', () => {
    it('should return core contract instance', () => {
      const contract = service.getCoreContract();
      expect(contract).toBeInstanceOf(Contract);
    });

    it('should return escrow contract instance', () => {
      const contract = service.getEscrowContract();
      expect(contract).toBeInstanceOf(Contract);
    });

    it('should return factory contract instance', () => {
      const contract = service.getFactoryContract();
      expect(contract).toBeInstanceOf(Contract);
    });

    it('should return auction contract instance', () => {
      const contract = service.getAuctionContract();
      expect(contract).toBeInstanceOf(Contract);
    });

    it('should return server instance', () => {
      const server = service.getServer();
      expect(server).toBe(mockServer);
    });
  });
});
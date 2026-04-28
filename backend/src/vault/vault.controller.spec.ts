import { Test, TestingModule } from '@nestjs/testing';
import { VaultController } from './vault.controller';

const VALID_ADDRESS = 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN';

describe('VaultController', () => {
  let controller: VaultController;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      controllers: [VaultController],
    }).compile();

    controller = module.get<VaultController>(VaultController);
  });

  describe('getBalance', () => {
    it('returns balance for a valid address', () => {
      const result = controller.getBalance(VALID_ADDRESS);
      expect(result.walletAddress).toBe(VALID_ADDRESS);
      expect(result.asset).toBe('XLM');
    });
  });

  describe('getPayments', () => {
    it('returns payment list for a valid address', () => {
      const result = controller.getPayments(VALID_ADDRESS);
      expect(Array.isArray(result)).toBe(true);
      expect(result[0]).toHaveProperty('txId');
    });
  });

  describe('getAutoPay', () => {
    it('returns auto-pay rules for a valid address', () => {
      const result = controller.getAutoPay(VALID_ADDRESS);
      expect(Array.isArray(result)).toBe(true);
      expect(result[0]).toHaveProperty('interval');
    });
  });
});

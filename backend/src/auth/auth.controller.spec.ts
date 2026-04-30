import { Test, TestingModule } from '@nestjs/testing';
import { AuthController } from './auth.controller';

const VALID_ADDRESS = 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN';

describe('AuthController', () => {
  let controller: AuthController;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      controllers: [AuthController],
    }).compile();

    controller = module.get<AuthController>(AuthController);
  });

  describe('getChallenge', () => {
    it('returns a challenge for a valid address in the body', () => {
      const result = controller.getChallenge({ address: VALID_ADDRESS });
      expect(result.address).toBe(VALID_ADDRESS);
      expect(result).toHaveProperty('challenge');
    });
  });

  describe('verifyChallenge', () => {
    it('returns authenticated true for a valid address param', () => {
      const result = controller.verifyChallenge(VALID_ADDRESS);
      expect(result.address).toBe(VALID_ADDRESS);
      expect(result.authenticated).toBe(true);
    });
  });
});

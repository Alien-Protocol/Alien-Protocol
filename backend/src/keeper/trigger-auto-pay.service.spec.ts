import { Test, TestingModule } from '@nestjs/testing';
import { getRepositoryToken } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { ConfigService } from '@nestjs/config';
import { Logger } from '@nestjs/common';
import { AutoPayRule } from './entities/auto-pay-rule.entity';
import { EscrowContractClient } from './escrow-contract.client';
import { TriggerAutoPayService } from './trigger-auto-pay.service';

describe('TriggerAutoPayService', () => {
  let service: TriggerAutoPayService;
  let escrowClient: jest.Mocked<EscrowContractClient>;
  let autoPayRepository: jest.Mocked<Repository<AutoPayRule>>;
  let queryBuilderMock: {
    where: jest.Mock;
    andWhere: jest.Mock;
    getMany: jest.Mock;
  };

  const mockAutoPayRule = (overrides: Partial<AutoPayRule> = {}): AutoPayRule =>
    ({
      ruleId: 1,
      fromCommitment: 'aabbccdd',
      toCommitment: '11223344',
      token: 'TOKEN',
      amount: '100',
      interval: '60',
      lastPaid: '0',
      isActive: true,
      needsAttention: false,
      ...overrides,
    } as AutoPayRule);

  beforeEach(async () => {
    queryBuilderMock = {
      where: jest.fn().mockReturnThis(),
      andWhere: jest.fn().mockReturnThis(),
      getMany: jest.fn().mockResolvedValue([]),
    };

    const escrowClientMock = {
      triggerAutoPay: jest.fn().mockResolvedValue(undefined),
    };

    const repositoryMock = {
      createQueryBuilder: jest.fn().mockReturnValue(queryBuilderMock),
      update: jest.fn().mockResolvedValue({ affected: 1 }),
    };

    const module: TestingModule = await Test.createTestingModule({
      providers: [
        TriggerAutoPayService,
        {
          provide: EscrowContractClient,
          useValue: escrowClientMock,
        },
        {
          provide: getRepositoryToken(AutoPayRule),
          useValue: repositoryMock,
        },
        {
          provide: ConfigService,
          useValue: {
            get: jest.fn((key: string) => {
              if (key === 'KEEPER_ENABLED') return 'true';
              if (key === 'KEEPER_SECRET_KEY') return 'SFAKESECRETKEY';
              return undefined;
            }),
          },
        },
      ],
    }).compile();

    service = module.get(TriggerAutoPayService);
    escrowClient = module.get(EscrowContractClient);
    autoPayRepository = module.get(getRepositoryToken(AutoPayRule));
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('should be defined', () => {
    expect(service).toBeDefined();
  });

  it('should skip execution when KEEPER_ENABLED is false', async () => {
    const qbMock = {
      where: jest.fn().mockReturnThis(),
      andWhere: jest.fn().mockReturnThis(),
      getMany: jest.fn().mockResolvedValue([]),
    };
    const repoMock = {
      createQueryBuilder: jest.fn().mockReturnValue(qbMock),
      update: jest.fn().mockResolvedValue({ affected: 1 }),
    };

    const module: TestingModule = await Test.createTestingModule({
      providers: [
        TriggerAutoPayService,
        {
          provide: EscrowContractClient,
          useValue: { triggerAutoPay: jest.fn() },
        },
        {
          provide: getRepositoryToken(AutoPayRule),
          useValue: repoMock,
        },
        {
          provide: ConfigService,
          useValue: {
            get: jest.fn(() => 'false'),
          },
        },
      ],
    }).compile();

    const disabledService = module.get(TriggerAutoPayService);
    await disabledService.handleDueRules();
    expect(repoMock.createQueryBuilder).not.toHaveBeenCalled();
  });

  it('should query only due active rules via SQL predicate', async () => {
    const rule = mockAutoPayRule();
    queryBuilderMock.getMany.mockResolvedValue([rule]);

    await service.handleDueRules();

    expect(autoPayRepository.createQueryBuilder).toHaveBeenCalledWith('r');
    expect(queryBuilderMock.where).toHaveBeenCalledWith(
      'r.isActive = :active',
      { active: true },
    );
    expect(queryBuilderMock.andWhere).toHaveBeenCalledWith(
      '(CAST(r.lastPaid AS INTEGER) + CAST(r.interval AS INTEGER)) <= :now',
      expect.objectContaining({ now: expect.any(Number) }),
    );
  });

  it('should trigger auto-pay for due rules and update lastPaid', async () => {
    const nowSeconds = Math.floor(Date.now() / 1000);
    const rule = mockAutoPayRule({
      lastPaid: String(nowSeconds - 100),
      interval: '60',
    });
    queryBuilderMock.getMany.mockResolvedValue([rule]);

    await service.handleDueRules();

    expect(escrowClient.triggerAutoPay).toHaveBeenCalledWith(
      'aabbccdd',
      1,
      'SFAKESECRETKEY',
    );
    expect(autoPayRepository.update).toHaveBeenCalledWith(
      { ruleId: 1 },
      { lastPaid: expect.any(String), needsAttention: false },
    );
  });

  it('should skip rules that are not yet due (filtered in SQL)', async () => {
    const nowSeconds = Math.floor(Date.now() / 1000);
    const rule = mockAutoPayRule({
      lastPaid: String(nowSeconds),
      interval: '3600',
    });
    // If the rule is not due, the SQL query should return nothing
    queryBuilderMock.getMany.mockResolvedValue([]);

    await service.handleDueRules();

    expect(escrowClient.triggerAutoPay).not.toHaveBeenCalled();
    expect(autoPayRepository.update).not.toHaveBeenCalled();
  });

  it('should log error, mark needsAttention, and continue on contract failure', async () => {
    const nowSeconds = Math.floor(Date.now() / 1000);
    const rule = mockAutoPayRule({
      ruleId: 99,
      fromCommitment: 'deadbeef',
      lastPaid: String(nowSeconds - 100),
      interval: '60',
    });
    queryBuilderMock.getMany.mockResolvedValue([rule]);
    escrowClient.triggerAutoPay.mockRejectedValue(new Error('InsufficientBalance'));

    const loggerSpy = jest
      .spyOn(Logger.prototype, 'error')
      .mockImplementation(() => {});

    await service.handleDueRules();

    expect(escrowClient.triggerAutoPay).toHaveBeenCalledWith(
      'deadbeef',
      99,
      'SFAKESECRETKEY',
    );
    expect(autoPayRepository.update).toHaveBeenCalledWith(
      { ruleId: 99 },
      { needsAttention: true },
    );
    expect(loggerSpy).toHaveBeenCalledWith(
      expect.stringContaining('Failed to trigger auto-pay for rule 99 from commitment deadbeef: InsufficientBalance'),
      expect.any(String),
    );
  });

  it('should skip overlapping runs', async () => {
    const rule = mockAutoPayRule({
      lastPaid: String(Math.floor(Date.now() / 1000) - 100),
      interval: '60',
    });
    queryBuilderMock.getMany.mockImplementation(async () => {
      await new Promise((r) => setTimeout(r, 100));
      return [rule];
    });

    const promise1 = service.handleDueRules();
    const promise2 = service.handleDueRules();

    await Promise.all([promise1, promise2]);

    expect(autoPayRepository.createQueryBuilder).toHaveBeenCalledTimes(1);
    expect(escrowClient.triggerAutoPay).toHaveBeenCalledTimes(1);
  });
});

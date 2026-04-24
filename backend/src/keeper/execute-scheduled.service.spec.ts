import { Test, TestingModule } from '@nestjs/testing';
import { getRepositoryToken } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { ConfigService } from '@nestjs/config';
import { Logger } from '@nestjs/common';
import { Payment } from './entities/payment.entity';
import { EscrowContractClient } from './escrow-contract.client';
import { ExecuteScheduledService } from './execute-scheduled.service';

describe('ExecuteScheduledService', () => {
  let service: ExecuteScheduledService;
  let escrowClient: jest.Mocked<EscrowContractClient>;
  let paymentRepository: jest.Mocked<Repository<Payment>>;

  const mockPayment = (overrides: Partial<Payment> = {}): Payment =>
    ({
      id: 1,
      paymentId: 100,
      executed: false,
      releaseAt: new Date('2024-01-01T00:00:00Z'),
      ...overrides,
    } as Payment);

  beforeEach(async () => {
    const escrowClientMock = {
      executeScheduled: jest.fn().mockResolvedValue(undefined),
    };

    const repositoryMock = {
      find: jest.fn().mockResolvedValue([]),
      save: jest.fn().mockImplementation((p) => Promise.resolve(p)),
    };

    const module: TestingModule = await Test.createTestingModule({
      providers: [
        ExecuteScheduledService,
        {
          provide: EscrowContractClient,
          useValue: escrowClientMock,
        },
        {
          provide: getRepositoryToken(Payment),
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

    service = module.get(ExecuteScheduledService);
    escrowClient = module.get(EscrowContractClient);
    paymentRepository = module.get(getRepositoryToken(Payment));
  });

  it('should be defined', () => {
    expect(service).toBeDefined();
  });

  it('should skip execution when KEEPER_ENABLED is false', async () => {
    const repoMock = { find: jest.fn(), save: jest.fn() };
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        ExecuteScheduledService,
        {
          provide: EscrowContractClient,
          useValue: { executeScheduled: jest.fn() },
        },
        {
          provide: getRepositoryToken(Payment),
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

    const disabledService = module.get(ExecuteScheduledService);
    await disabledService.handleDuePayments();
    expect(repoMock.find).not.toHaveBeenCalled();
  });

  it('should query only due unexecuted payments', async () => {
    const payment = mockPayment({ releaseAt: new Date(Date.now() - 1000) });
    paymentRepository.find.mockResolvedValue([payment]);

    await service.handleDuePayments();

    expect(paymentRepository.find).toHaveBeenCalledTimes(1);
    const arg = paymentRepository.find.mock.calls[0][0];
    expect(arg?.where).toMatchObject({ executed: false });
    const releaseAtOp: any = (arg?.where as any).releaseAt;
    expect(releaseAtOp?._type ?? releaseAtOp?.type).toBe('lessThanOrEqual');
    const boundary: Date = releaseAtOp?._value ?? releaseAtOp?.value;
    expect(boundary).toBeInstanceOf(Date);
    expect(boundary.getTime()).toBeLessThanOrEqual(Date.now());
  });

  it('should execute scheduled payment and mark as executed', async () => {
    const payment = mockPayment({ paymentId: 42 });
    paymentRepository.find.mockResolvedValue([payment]);

    await service.handleDuePayments();

    expect(escrowClient.executeScheduled).toHaveBeenCalledWith(42, 'SFAKESECRETKEY');
    expect(payment.executed).toBe(true);
    expect(paymentRepository.save).toHaveBeenCalledWith(payment);
  });

  it('should log error and skip on contract failure', async () => {
    const payment = mockPayment({ paymentId: 99 });
    paymentRepository.find.mockResolvedValue([payment]);
    escrowClient.executeScheduled.mockRejectedValue(new Error('contract failed'));

    const loggerSpy = jest
      .spyOn(Logger.prototype, 'error')
      .mockImplementation(() => {});

    await service.handleDuePayments();

    expect(escrowClient.executeScheduled).toHaveBeenCalledWith(99, 'SFAKESECRETKEY');
    expect(payment.executed).toBe(false);
    expect(paymentRepository.save).not.toHaveBeenCalled();
    expect(loggerSpy).toHaveBeenCalledWith(
      expect.stringContaining('Failed to execute payment 99: contract failed'),
      expect.any(String),
    );

    loggerSpy.mockRestore();
  });

  it('should skip overlapping runs', async () => {
    const payment = mockPayment({ paymentId: 1 });
    paymentRepository.find.mockImplementation(async () => {
      await new Promise((r) => setTimeout(r, 100));
      return [payment];
    });

    const promise1 = service.handleDuePayments();
    const promise2 = service.handleDuePayments();

    await Promise.all([promise1, promise2]);

    // Only one run should have queried the repository
    expect(paymentRepository.find).toHaveBeenCalledTimes(1);
  });
});

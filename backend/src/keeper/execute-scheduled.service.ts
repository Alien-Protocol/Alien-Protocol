import { Injectable, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { Interval } from '@nestjs/schedule';
import { InjectRepository } from '@nestjs/typeorm';
import { LessThanOrEqual, Repository } from 'typeorm';
import { Payment } from './entities/payment.entity';
import { EscrowContractClient } from './escrow-contract.client';

@Injectable()
export class ExecuteScheduledService {
  private readonly logger = new Logger(ExecuteScheduledService.name);
  private readonly enabled: boolean;
  private readonly secretKey: string | undefined;

  constructor(
    private readonly configService: ConfigService,
    private readonly escrowClient: EscrowContractClient,
    @InjectRepository(Payment)
    private readonly paymentRepository: Repository<Payment>,
  ) {
    this.enabled = this.configService.get<string>('KEEPER_ENABLED') === 'true';
    this.secretKey = this.configService.get<string>('KEEPER_SECRET_KEY');
  }

  @Interval(30000)
  async handleDuePayments(): Promise<void> {
    if (!this.enabled) {
      return;
    }

    if (!this.secretKey) {
      this.logger.warn('KEEPER_SECRET_KEY is not set, skipping execution');
      return;
    }

    const now = new Date();
    const duePayments = await this.paymentRepository.find({
      where: {
        executed: false,
        releaseAt: LessThanOrEqual(now),
      },
    });

    for (const payment of duePayments) {
      try {
        await this.escrowClient.executeScheduled(payment.paymentId, this.secretKey);
        payment.executed = true;
        await this.paymentRepository.save(payment);
        this.logger.log(`Executed scheduled payment ${payment.paymentId}`);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        this.logger.error(`Failed to execute payment ${payment.paymentId}: ${message}`);
      }
    }
  }
}

import { Injectable, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { Interval } from '@nestjs/schedule';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { AutoPayRule } from './entities/auto-pay-rule.entity';
import { EscrowContractClient } from './escrow-contract.client';

@Injectable()
export class TriggerAutoPayService {
  private readonly logger = new Logger(TriggerAutoPayService.name);
  private readonly enabled: boolean;
  private readonly secretKey: string | undefined;
  private isRunning = false;

  constructor(
    private readonly configService: ConfigService,
    private readonly escrowClient: EscrowContractClient,
    @InjectRepository(AutoPayRule)
    private readonly autoPayRepository: Repository<AutoPayRule>,
  ) {
    this.enabled = this.configService.get<string>('KEEPER_ENABLED') === 'true';
    this.secretKey = this.configService.get<string>('KEEPER_SECRET_KEY');
  }

  @Interval(60000)
  async handleDueRules(): Promise<void> {
    if (!this.enabled) {
      return;
    }

    if (!this.secretKey) {
      this.logger.warn('KEEPER_SECRET_KEY is not set, skipping auto-pay execution');
      return;
    }

    if (this.isRunning) {
      this.logger.warn('Previous auto-pay keeper run still in progress, skipping');
      return;
    }

    this.isRunning = true;
    try {
      const nowSeconds = Math.floor(Date.now() / 1000);
      const dueRules = await this.autoPayRepository
        .createQueryBuilder('r')
        .where('r.isActive = :active', { active: true })
        .andWhere(
          '(CAST(r.lastPaid AS INTEGER) + CAST(r.interval AS INTEGER)) <= :now',
          { now: nowSeconds },
        )
        .getMany();

      for (const rule of dueRules) {
        try {
          this.logger.log(
            `Triggering auto-pay for rule ${rule.ruleId} from commitment ${rule.fromCommitment}`,
          );
          await this.escrowClient.triggerAutoPay(
            rule.fromCommitment,
            rule.ruleId,
            this.secretKey,
          );
          await this.autoPayRepository.update(
            { ruleId: rule.ruleId },
            { lastPaid: String(nowSeconds), needsAttention: false },
          );
          this.logger.log(`Auto-pay triggered successfully for rule ${rule.ruleId}`);
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          const stack = error instanceof Error ? error.stack : undefined;
          this.logger.error(
            `Failed to trigger auto-pay for rule ${rule.ruleId} from commitment ${rule.fromCommitment}: ${message}`,
            stack,
          );
          try {
            await this.autoPayRepository.update(
              { ruleId: rule.ruleId },
              { needsAttention: true },
            );
          } catch (dbError) {
            const dbMessage = dbError instanceof Error ? dbError.message : String(dbError);
            this.logger.error(
              `Failed to persist attention flag for rule ${rule.ruleId}: ${dbMessage}`,
            );
          }
        }
      }
    } finally {
      this.isRunning = false;
    }
  }
}

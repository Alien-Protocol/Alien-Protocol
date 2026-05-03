import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { Payment } from './entities/payment.entity';
import { AutoPayRule } from './entities/auto-pay-rule.entity';
import { EscrowContractClient } from './escrow-contract.client';
import { ExecuteScheduledService } from './execute-scheduled.service';
import { TriggerAutoPayService } from './trigger-auto-pay.service';

@Module({
  imports: [TypeOrmModule.forFeature([Payment, AutoPayRule])],
  providers: [EscrowContractClient, ExecuteScheduledService, TriggerAutoPayService],
})
export class KeeperModule {}

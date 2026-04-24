import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { Payment } from './entities/payment.entity';
import { EscrowContractClient } from './escrow-contract.client';
import { ExecuteScheduledService } from './execute-scheduled.service';

@Module({
  imports: [TypeOrmModule.forFeature([Payment])],
  providers: [EscrowContractClient, ExecuteScheduledService],
})
export class KeeperModule {}

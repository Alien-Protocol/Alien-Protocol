import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { Auction } from './entities/auction.entity';
import { AuctionContractClient } from './auction-contract.client';
import { AuctionService } from './auction.service';
import { AuctionController } from './auction.controller';
import { StellarModule } from '../stellar/stellar.module';

@Module({
  imports: [
    TypeOrmModule.forFeature([Auction]),
    StellarModule,
  ],
  providers: [AuctionContractClient, AuctionService],
  controllers: [AuctionController],
  exports: [AuctionService],
})
export class AuctionModule {}
import { Module } from '@nestjs/common';
import { AuctionController } from './auction.controller';

@Module({ controllers: [AuctionController] })
export class AuctionModule {}

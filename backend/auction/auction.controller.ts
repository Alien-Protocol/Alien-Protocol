import { Controller, Get, Param } from '@nestjs/common';
import { AuctionService } from './auction.service';
import { AuctionInfoDto, AuctionBidsDto, AuctionBidDto } from './dto/auction.dto';

@Controller('auction')
export class AuctionController {
  constructor(private readonly auctionService: AuctionService) {}

  /** GET /auction/:id — full auction info */
  @Get(':id')
  getAuctionInfo(@Param('id') id: string): Promise<AuctionInfoDto> {
    return this.auctionService.getAuctionInfo(id);
  }

  /** GET /auction/:id/bids — all bidder addresses */
  @Get(':id/bids')
  getAuctionBids(@Param('id') id: string): Promise<AuctionBidsDto> {
    return this.auctionService.getAuctionBids(id);
  }

  /** GET /auction/:id/bid/:bidder — specific bid amount */
  @Get(':id/bid/:bidder')
  getBid(
    @Param('id') id: string,
    @Param('bidder') bidder: string,
  ): Promise<AuctionBidDto> {
    return this.auctionService.getBid(id, bidder);
  }

  /** GET /auction/username/:usernameHash — auction for a username hash */
  @Get('username/:usernameHash')
  getAuctionByUsernameHash(
    @Param('usernameHash') usernameHash: string,
  ): Promise<AuctionInfoDto> {
    return this.auctionService.getAuctionByUsernameHash(usernameHash);
  }
}
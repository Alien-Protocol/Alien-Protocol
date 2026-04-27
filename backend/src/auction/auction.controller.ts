import { Body, Controller, Get, Param, Post } from '@nestjs/common';
import { ApiOperation, ApiParam, ApiResponse, ApiTags } from '@nestjs/swagger';
import { AuctionInfoDto, BidResponseDto, PlaceBidDto } from './dto/auction.dto';

@ApiTags('auction')
@Controller('auction')
export class AuctionController {
  @Get(':id')
  @ApiOperation({ summary: 'Get auction info by ID' })
  @ApiParam({ name: 'id', description: 'Auction ID', example: 1 })
  @ApiResponse({ status: 200, description: 'Auction info retrieved', type: AuctionInfoDto })
  @ApiResponse({ status: 404, description: 'Auction not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  getAuction(@Param('id') id: number): AuctionInfoDto {
    return {
      id,
      username: 'satoshi',
      seller: 'GABC1234STELLAR5678WALLETADDRESS',
      minBid: '100.00',
      currentBid: '150.00',
      highestBidder: 'GXYZ9876STELLAR1234BIDDERADDRESS',
      endTime: 1745000000,
      status: 'open',
    };
  }

  @Post(':id/bid')
  @ApiOperation({ summary: 'Place a bid on an auction' })
  @ApiParam({ name: 'id', description: 'Auction ID', example: 1 })
  @ApiResponse({ status: 201, description: 'Bid placed successfully', type: BidResponseDto })
  @ApiResponse({ status: 400, description: 'Bid too low or auction closed' })
  @ApiResponse({ status: 404, description: 'Auction not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  placeBid(@Param('id') id: number, @Body() dto: PlaceBidDto): BidResponseDto {
    return { success: true, txHash: 'tx_bid123' };
  }

  @Post(':id/claim')
  @ApiOperation({ summary: 'Claim a username after winning an auction' })
  @ApiParam({ name: 'id', description: 'Auction ID', example: 1 })
  @ApiResponse({ status: 201, description: 'Username claimed successfully', type: BidResponseDto })
  @ApiResponse({ status: 400, description: 'Auction not closed or caller is not the winner' })
  @ApiResponse({ status: 404, description: 'Auction not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  claim(@Param('id') id: number): BidResponseDto {
    return { success: true, txHash: 'tx_claim456' };
  }
}

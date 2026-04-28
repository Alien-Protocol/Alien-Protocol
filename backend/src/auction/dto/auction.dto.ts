import { IsNotEmpty, IsNumberString, Matches } from 'class-validator';
import { ApiProperty } from '@nestjs/swagger';

export class AuctionInfoDto {
  @ApiProperty({ description: 'Auction ID', example: 1 })
  id: number;

  @ApiProperty({ description: 'Username being auctioned', example: 'satoshi' })
  username: string;

  @ApiProperty({ description: 'Seller Stellar address', example: 'GABC1234STELLAR5678WALLETADDRESS' })
  seller: string;

  @ApiProperty({ description: 'Minimum bid in XLM', example: '100.00' })
  minBid: string;

  @ApiProperty({ description: 'Current highest bid in XLM', example: '150.00' })
  currentBid: string;

  @ApiProperty({ description: 'Current highest bidder address', example: 'GXYZ9876STELLAR1234BIDDERADDRESS' })
  highestBidder: string;

  @ApiProperty({ description: 'Auction end time (Unix timestamp)', example: 1745000000 })
  endTime: number;

  @ApiProperty({ description: 'Auction status', example: 'open', enum: ['open', 'closed', 'claimed'] })
  status: string;
}

export class PlaceBidDto {
  @ApiProperty({ description: 'Bidder Stellar address', example: 'GXYZ9876STELLAR1234BIDDERADDRESS' })
  @IsNotEmpty()
  @Matches(/^G[A-Z2-7]{55}$/, {
    message: 'Invalid Stellar wallet address format',
  })
  bidder: string;

  @ApiProperty({ description: 'Bid amount in XLM', example: '150.00' })
  @IsNotEmpty()
  @IsNumberString()
  amount: string;
}


export class BidResponseDto {
  @ApiProperty({ description: 'Whether the bid was accepted', example: true })
  success: boolean;

  @ApiProperty({ description: 'Transaction hash', example: 'tx_bid123' })
  txHash: string;
}

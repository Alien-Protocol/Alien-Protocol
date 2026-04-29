import { Controller, Get, Param, Query } from '@nestjs/common';
import { ApiOperation, ApiParam, ApiQuery, ApiResponse, ApiTags } from '@nestjs/swagger';
import { StellarAddressPipe } from './stellar-address.pipe';
import { AuctionListItemDto } from '../auction/dto/auction.dto';
import { StellarService } from './stellar.service';

@ApiTags('stellar')
@Controller('stellar')
export class StellarController {
  constructor(private readonly stellarService: StellarService) {}

  @Get(':address/account')
  @ApiOperation({ summary: 'Get Stellar account info for a given address' })
  @ApiParam({
    name: 'address',
    description: 'Stellar wallet address (56 chars, starts with G)',
    example: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
  })
  @ApiResponse({ status: 200, description: 'Account info returned' })
  @ApiResponse({
    status: 400,
    description: 'Invalid Stellar address format',
  })
  getAccount(@Param('address', StellarAddressPipe) address: string) {
    return { address };
  }

  @Get(':address/balance')
  @ApiOperation({ summary: 'Get native XLM balance for a Stellar address' })
  @ApiParam({
    name: 'address',
    description: 'Stellar wallet address (56 chars, starts with G)',
    example: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
  })
  @ApiResponse({ status: 200, description: 'Balance returned' })
  @ApiResponse({
    status: 400,
    description: 'Invalid Stellar address format',
  })
  getBalance(@Param('address', StellarAddressPipe) address: string) {
    return { address, balance: '0', asset: 'XLM' };
  }

  @Get('auctions')
  @ApiOperation({ summary: 'Get paginated list of auctions with optional status filter' })
  @ApiQuery({
    name: 'page',
    description: 'Page number (1-indexed)',
    required: false,
    type: Number,
    example: 1,
  })
  @ApiQuery({
    name: 'limit',
    description: 'Number of auctions per page (max 100, default 20)',
    required: false,
    type: Number,
    example: 20,
  })
  @ApiQuery({
    name: 'status',
    description: 'Filter by auction status (open, closed, claimed)',
    required: false,
    type: String,
    enum: ['open', 'closed', 'claimed'],
  })
  @ApiResponse({ status: 200, description: 'Auction list retrieved', type: [AuctionListItemDto] })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  async getAuctions(
    @Query('page') page?: string,
    @Query('limit') limit?: string,
    @Query('status') status?: string,
  ): Promise<AuctionListItemDto[]> {
    const p = page ? parseInt(page, 10) : 1;
    const l = limit ? Math.min(parseInt(limit, 10), 100) : 20;
    return this.stellarService.getAuctions(p, l, status);
  }
}

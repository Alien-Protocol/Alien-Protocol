import { Controller, Get, Param } from '@nestjs/common';
import { ApiOperation, ApiParam, ApiResponse, ApiTags } from '@nestjs/swagger';
import { StellarAddressPipe } from './stellar-address.pipe';

@ApiTags('stellar')
@Controller('stellar')
export class StellarController {
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
}

import { Controller, Get, Param } from '@nestjs/common';
import { ApiOperation, ApiParam, ApiResponse, ApiTags } from '@nestjs/swagger';
import { AutoPayDto, PaymentDto, VaultBalanceDto, VaultListItemDto } from './dto/vault.dto';
import { StellarAddressPipe } from '../stellar/stellar-address.pipe';
import { VaultService } from './vault.service';

@ApiTags('vault')
@Controller('vault')
export class VaultController {
  constructor(private readonly vaultService: VaultService) {}

  @Get(':address/balance')
  @ApiOperation({ summary: 'Get vault balance for a Stellar address' })
  @ApiParam({
    name: 'address',
    description: 'Stellar wallet address (56 chars, starts with G)',
    example: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
  })
  @ApiResponse({ status: 200, description: 'Balance retrieved successfully', type: VaultBalanceDto })
  @ApiResponse({ status: 400, description: 'Invalid Stellar address format' })
  @ApiResponse({ status: 404, description: 'Address not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  getBalance(@Param('address', StellarAddressPipe) address: string): VaultBalanceDto {
    return { walletAddress: address, balance: '250.50', asset: 'XLM' };
  }

  @Get(':address/payments')
  @ApiOperation({ summary: 'Get payment history for a Stellar address' })
  @ApiParam({
    name: 'address',
    description: 'Stellar wallet address (56 chars, starts with G)',
    example: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
  })
  @ApiResponse({ status: 200, description: 'Payment history retrieved', type: [PaymentDto] })
  @ApiResponse({ status: 400, description: 'Invalid Stellar address format' })
  @ApiResponse({ status: 404, description: 'Address not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  getPayments(@Param('address', StellarAddressPipe) address: string): PaymentDto[] {
    return [{ txId: 'tx_abc123', from: 'alice', to: 'bob', amount: '10.00', timestamp: '2026-04-24T05:00:00Z' }];
  }

   @Get(':address/autopay')
  @ApiOperation({ summary: 'Get auto-pay rules for a Stellar address' })
  @ApiParam({
    name: 'address',
    description: 'Stellar wallet address (56 chars, starts with G)',
    example: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
  })
  @ApiResponse({ status: 200, description: 'Auto-pay rules retrieved', type: [AutoPayDto] })
  @ApiResponse({ status: 400, description: 'Invalid Stellar address format' })
  @ApiResponse({ status: 404, description: 'Address not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  getAutoPay(@Param('address', StellarAddressPipe) address: string): AutoPayDto[] {
    return [{ id: 'ap_xyz789', recipient: 'bob', amount: '5.00', interval: 'monthly', active: true }];
  }

  @Get('users/:id/vaults')
  @ApiOperation({ summary: 'Get all vaults owned by a user' })
  @ApiParam({
    name: 'id',
    description: 'User Stellar address (56 chars, starts with G)',
    example: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
  })
  @ApiResponse({ status: 200, description: 'Vault list retrieved', type: [VaultListItemDto] })
  @ApiResponse({ status: 404, description: 'User not found or has no vaults' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  async getVaultsByUser(@Param('id', StellarAddressPipe) id: string): Promise<VaultListItemDto[]> {
    return this.vaultService.getVaultsByOwner(id);
  }
}


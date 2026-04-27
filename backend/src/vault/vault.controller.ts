import { Controller, Get, Param } from '@nestjs/common';
import { ApiOperation, ApiParam, ApiResponse, ApiTags } from '@nestjs/swagger';
import { AutoPayDto, PaymentDto, VaultBalanceDto } from './dto/vault.dto';

@ApiTags('vault')
@Controller('vault')
export class VaultController {
  @Get(':address/balance')
  @ApiOperation({ summary: 'Get vault balance for a Stellar address' })
  @ApiParam({ name: 'address', description: 'Stellar wallet address', example: 'GABC1234STELLAR5678WALLETADDRESS' })
  @ApiResponse({ status: 200, description: 'Balance retrieved successfully', type: VaultBalanceDto })
  @ApiResponse({ status: 404, description: 'Address not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  getBalance(@Param('address') address: string): VaultBalanceDto {
    return { walletAddress: address, balance: '250.50', asset: 'XLM' };
  }

  @Get(':address/payments')
  @ApiOperation({ summary: 'Get payment history for a Stellar address' })
  @ApiParam({ name: 'address', description: 'Stellar wallet address', example: 'GABC1234STELLAR5678WALLETADDRESS' })
  @ApiResponse({ status: 200, description: 'Payment history retrieved', type: [PaymentDto] })
  @ApiResponse({ status: 404, description: 'Address not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  getPayments(@Param('address') address: string): PaymentDto[] {
    return [{ txId: 'tx_abc123', from: 'alice', to: 'bob', amount: '10.00', timestamp: '2026-04-24T05:00:00Z' }];
  }

  @Get(':address/autopay')
  @ApiOperation({ summary: 'Get auto-pay rules for a Stellar address' })
  @ApiParam({ name: 'address', description: 'Stellar wallet address', example: 'GABC1234STELLAR5678WALLETADDRESS' })
  @ApiResponse({ status: 200, description: 'Auto-pay rules retrieved', type: [AutoPayDto] })
  @ApiResponse({ status: 404, description: 'Address not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  getAutoPay(@Param('address') address: string): AutoPayDto[] {
    return [{ id: 'ap_xyz789', recipient: 'bob', amount: '5.00', interval: 'monthly', active: true }];
  }
}

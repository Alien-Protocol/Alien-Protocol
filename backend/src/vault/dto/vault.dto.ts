import { IsBoolean, IsIn, IsNotEmpty, IsNumberString, IsString } from 'class-validator';
import { ApiProperty } from '@nestjs/swagger';

export class VaultBalanceDto {
  @ApiProperty({ description: 'Stellar wallet address', example: 'GABC1234STELLAR5678WALLETADDRESS' })
  walletAddress: string;

  @ApiProperty({ description: 'Balance in XLM', example: '250.50' })
  balance: string;

  @ApiProperty({ description: 'Asset type', example: 'XLM' })
  asset: string;
}

export class PaymentDto {
  @ApiProperty({ description: 'Payment transaction ID', example: 'tx_abc123' })
  txId: string;

  @ApiProperty({ description: 'Sender username or address', example: 'alice' })
  from: string;

  @ApiProperty({ description: 'Recipient username or address', example: 'bob' })
  to: string;

  @ApiProperty({ description: 'Amount in XLM', example: '10.00' })
  amount: string;

  @ApiProperty({ description: 'ISO timestamp of the payment', example: '2026-04-24T05:00:00Z' })
  timestamp: string;
}

export class AutoPayDto {
  @ApiProperty({ description: 'Auto-pay rule ID', example: 'ap_xyz789' })
  @IsString()
  @IsNotEmpty()
  id: string;

  @ApiProperty({ description: 'Recipient username', example: 'bob' })
  @IsString()
  @IsNotEmpty()
  recipient: string;

  @ApiProperty({ description: 'Amount per interval in XLM', example: '5.00' })
  @IsNotEmpty()
  @IsNumberString()
  amount: string;

  @ApiProperty({ description: 'Payment interval', example: 'monthly', enum: ['daily', 'weekly', 'monthly'] })
  @IsNotEmpty()
  @IsIn(['daily', 'weekly', 'monthly'])
  interval: string;

  @ApiProperty({ description: 'Whether the rule is active', example: true })
  @IsBoolean()
  active: boolean;
}


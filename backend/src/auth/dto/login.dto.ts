import { ApiProperty } from '@nestjs/swagger';
import { IsNotEmpty, IsString, Matches } from 'class-validator';

export class LoginDto {
  @ApiProperty({ description: 'Stellar wallet address', example: 'GABC1234STELLAR5678WALLETADDRESS' })
  @IsString()
  @IsNotEmpty()
  @Matches(/^G[A-Z2-7]{55}$/, {
    message: 'Invalid Stellar wallet address format',
  })
  address: string;

  @ApiProperty({ description: 'Signature for authentication', example: 'sig_abc123...' })
  @IsString()
  @IsNotEmpty()
  signature: string;
}

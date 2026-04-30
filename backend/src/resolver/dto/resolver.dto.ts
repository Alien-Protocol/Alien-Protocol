import { IsNotEmpty, IsString, Matches } from 'class-validator';
import { ApiProperty } from '@nestjs/swagger';

export class ResolveResponseDto {
  @ApiProperty({ description: 'Stellar wallet address linked to the username', example: 'GABC1234STELLAR5678WALLETADDRESS' })
  walletAddress: string;

  @ApiProperty({ description: 'Resolved username', example: 'alice' })
  username: string;

  @ApiProperty({ description: 'Whether the username is publicly visible', example: true })
  isPublic: boolean;
}

export class RegisterUsernameDto {
  @ApiProperty({ description: 'Username to register (without @)', example: 'alice' })
  @IsString()
  @IsNotEmpty()
  @Matches(/^[a-zA-Z0-9_]{3,20}$/, {
    message: 'Username must be 3-20 characters long and contain only letters, numbers, and underscores',
  })
  username: string;

  @ApiProperty({ description: 'Stellar wallet address to link', example: 'GABC1234STELLAR5678WALLETADDRESS' })
  @IsString()
  @IsNotEmpty()
  @Matches(/^G[A-Z2-7]{55}$/, {
    message: 'Invalid Stellar wallet address format',
  })
  walletAddress: string;

  @ApiProperty({ description: 'Zero-knowledge commitment hash', example: '0xabc123...' })
  @IsString()
  @IsNotEmpty()
  @Matches(/^0x[a-fA-F0-9]{64}$/, {
    message: 'Commitment must be a 32-byte hex string prefixed with 0x',
  })
  commitment: string;
}


export class RegisterResponseDto {
  @ApiProperty({ description: 'Whether registration succeeded', example: true })
  success: boolean;

  @ApiProperty({ description: 'Transaction hash on Stellar', example: 'abc123txhash' })
  txHash: string;
}

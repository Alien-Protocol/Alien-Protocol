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
  username: string;

  @ApiProperty({ description: 'Stellar wallet address to link', example: 'GABC1234STELLAR5678WALLETADDRESS' })
  walletAddress: string;

  @ApiProperty({ description: 'Zero-knowledge commitment hash', example: '0xabc123...' })
  commitment: string;
}

export class RegisterResponseDto {
  @ApiProperty({ description: 'Whether registration succeeded', example: true })
  success: boolean;

  @ApiProperty({ description: 'Transaction hash on Stellar', example: 'abc123txhash' })
  txHash: string;
}

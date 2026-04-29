import { Controller, Post, Body, Param } from '@nestjs/common';
import { ApiOperation, ApiParam, ApiResponse, ApiTags } from '@nestjs/swagger';
import { IsString, Matches } from 'class-validator';
import { ApiProperty } from '@nestjs/swagger';
import { StellarAddressPipe } from '../stellar/stellar-address.pipe';

export class AuthChallengeDto {
  @ApiProperty({
    description: 'Stellar wallet address requesting a challenge',
    example: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
  })
  @IsString()
  @Matches(/^G[A-Z2-7]{55}$/, {
    message:
      'address must be a 56-character Stellar public key starting with G',
  })
  address: string;
}

@ApiTags('auth')
@Controller('auth')
export class AuthController {
  @Post('challenge')
  @ApiOperation({ summary: 'Request a sign-in challenge for a Stellar address' })
  @ApiResponse({ status: 201, description: 'Challenge issued' })
  @ApiResponse({ status: 400, description: 'Invalid Stellar address format' })
  getChallenge(@Body() dto: AuthChallengeDto) {
    return { address: dto.address, challenge: 'sign-this-nonce-placeholder' };
  }

  @Post(':address/verify')
  @ApiOperation({ summary: 'Verify a signed challenge for a Stellar address' })
  @ApiParam({
    name: 'address',
    description: 'Stellar wallet address (56 chars, starts with G)',
    example: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
  })
  @ApiResponse({ status: 200, description: 'Authentication successful' })
  @ApiResponse({ status: 400, description: 'Invalid Stellar address format' })
  verifyChallenge(@Param('address', StellarAddressPipe) address: string) {
    return { address, authenticated: true };
  }
}

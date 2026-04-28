import { Body, Controller, Get, Param, Post } from '@nestjs/common';
import { ApiOperation, ApiParam, ApiResponse, ApiTags } from '@nestjs/swagger';
import { RegisterResponseDto, RegisterUsernameDto, ResolveResponseDto } from './dto/resolver.dto';

@ApiTags('resolver')
@Controller('resolver')
export class ResolverController {
  @Get(':username')
  @ApiOperation({ summary: 'Resolve a username to a Stellar wallet address' })
  @ApiParam({ name: 'username', description: 'Username to resolve (without @)', example: 'alice' })
  @ApiResponse({ status: 200, description: 'Username resolved successfully', type: ResolveResponseDto })
  @ApiResponse({ status: 404, description: 'Username not found' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  resolve(@Param('username') username: string): ResolveResponseDto {
    return { walletAddress: 'GABC1234STELLAR5678WALLETADDRESS', username, isPublic: true };
  }

  @Post('register')
  @ApiOperation({ summary: 'Register a new username with a ZK commitment' })
  @ApiResponse({ status: 201, description: 'Username registered successfully', type: RegisterResponseDto })
  @ApiResponse({ status: 400, description: 'Invalid input or username already taken' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  register(@Body() dto: RegisterUsernameDto): RegisterResponseDto {
    return { success: true, txHash: 'abc123txhash' };
  }
}

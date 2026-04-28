import { Module } from '@nestjs/common';
import { ApiKeyGuard } from './guards/api-key.guard';
import { AuthController } from './auth.controller';

@Module({
  controllers: [AuthController],
  providers: [ApiKeyGuard],
  exports: [ApiKeyGuard],
})
export class AuthModule {}

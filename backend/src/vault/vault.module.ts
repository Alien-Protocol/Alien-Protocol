import { Module } from '@nestjs/common';
import { VaultController } from './vault.controller';
import { VaultService } from './vault.service';
import { PrismaService } from '../prisma.service';
import { SorobanService } from '../soroban.service';

@Module({
  controllers: [VaultController],
  providers: [VaultService, PrismaService, SorobanService],
})
export class VaultModule {}

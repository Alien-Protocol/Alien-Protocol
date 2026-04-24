import { Module } from '@nestjs/common';
import { VaultController } from './vault.controller';

@Module({ controllers: [VaultController] })
export class VaultModule {}

import { Module } from '@nestjs/common';
import { VaultController } from './vault.controller';
import { VaultService } from './vault.service';

@Module({ controllers: [VaultController], providers: [VaultService] })
export class VaultModule {}

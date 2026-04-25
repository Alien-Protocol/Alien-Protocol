import { Module } from '@nestjs/common';
import { VaultModule } from './vault/vault.module';

@Module({
  imports: [VaultModule],
})
export class AppModule {}

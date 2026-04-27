import { Module } from '@nestjs/common';
import { ResolverModule } from './resolver/resolver.module';
import { VaultModule } from './vault/vault.module';
import { AuctionModule } from './auction/auction.module';

@Module({
  imports: [ResolverModule, VaultModule, AuctionModule],
})
export class AppModule {}

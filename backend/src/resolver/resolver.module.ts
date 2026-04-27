import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { Username } from '../username/username.entity';
import { ResolverController } from './resolver.controller';
import { ResolverService } from './resolver.service';

@Module({
  imports: [TypeOrmModule.forFeature([Username])],
  controllers: [ResolverController],
  providers: [ResolverService],
  exports: [ResolverService],
})
export class ResolverModule {}

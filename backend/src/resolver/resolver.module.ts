import { Module } from '@nestjs/common';
import { ResolverController } from './resolver.controller';

@Module({ controllers: [ResolverController] })
export class ResolverModule {}

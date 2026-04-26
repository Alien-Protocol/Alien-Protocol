import { Module } from '@nestjs/common';
import { APP_GUARD } from '@nestjs/core';
import { ConfigModule, ConfigService } from '@nestjs/config';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ScheduleModule } from '@nestjs/schedule';

import { AppController } from './app.controller';
import { AppService } from './app.service';
import { VaultModule } from './vault/vault.module';
import { KeeperModule } from './keeper/keeper.module';
import { AuthModule } from './auth/auth.module';
import { ApiKeyGuard } from './auth/guards/api-key.guard';

@Module({
  imports: [
    ConfigModule.forRoot({ isGlobal: true }),
    ScheduleModule.forRoot(),
    TypeOrmModule.forRootAsync({
      imports: [ConfigModule],
      useFactory: (configService: ConfigService) => ({
        type: 'sqlite',
        database: configService.get<string>('DATABASE_URL') || './data.sqlite',
        entities: [__dirname + '/**/*.entity{.ts,.js}'],
        synchronize: configService.get<string>('TYPEORM_SYNCHRONIZE') === 'true',
      }),
      inject: [ConfigService],
    }),
    VaultModule,
    KeeperModule,
    AuthModule,
  ],
  controllers: [AppController],
  providers: [
    AppService,
    {
      provide: APP_GUARD,
      useClass: ApiKeyGuard,
    },
  ],
})
export class AppModule {}

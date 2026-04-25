import { Module } from '@nestjs/common';

import { DatabaseModule } from './database/database.module';
import { AppController } from './app.controller';
import { AppService } from './app.service';
import { ConfigModule } from './config/config.module';
import { ConfigService } from './config/config.service';
import { StellarModule } from './stellar/stellar.module';

@Module({
  imports: [
    ConfigModule,
    DatabaseModule,
    StellarModule,
  ],
  controllers: [AppController],

import { ConfigModule, ConfigService } from '@nestjs/config';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ScheduleModule } from '@nestjs/schedule';
import { KeeperModule } from './keeper/keeper.module';

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
    KeeperModule,
  ],

})
export class AppModule {}

import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ConfigModule } from '../config/config.module';
import { ConfigService } from '../config/config.service';
import { Username, Vault, Payment, AutoPayRule } from './entities';

@Module({
  imports: [
    TypeOrmModule.forRootAsync({
      imports: [ConfigModule],
      inject: [ConfigService],
      useFactory: (configService: ConfigService) => ({
        type: 'postgres',
        url: configService.databaseUrl,
        entities: [Username, Vault, Payment, AutoPayRule],
        synchronize: false, // Use migrations instead
        migrations: [__dirname + '/migrations/**/*{.ts,.js}'],
        migrationsRun: true,
        logging: true,
      }),
    }),
    TypeOrmModule.forFeature([Username, Vault, Payment, AutoPayRule]),
  ],
  exports: [TypeOrmModule],
})
export class DatabaseModule {}

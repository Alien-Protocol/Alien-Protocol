import 'reflect-metadata';
import { DataSource } from 'typeorm';
import { config } from 'dotenv';
import { Username, Vault, Payment, AutoPayRule } from './entities';

config();

export const AppDataSource = new DataSource({
  type: 'postgres',
  url: process.env.DATABASE_URL,
  synchronize: false,
  logging: true,
  entities: [Username, Vault, Payment, AutoPayRule],
  migrations: [__dirname + '/migrations/**/*{.ts,.js}'],
  subscribers: [],
});

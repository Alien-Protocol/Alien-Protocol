import { NestFactory } from '@nestjs/core';
p
import { ValidationPipe } from '@nestjs/common';
import { AppModule } from './app.module';

async function bootstrap() {
  const app = await NestFactory.create(AppModule);
  app.useGlobalPipes(new ValidationPipe({
    whitelist: true,
    transform: true,
    forbidNonWhitelisted: true,
  }));
  await app.listen(process.env.PORT ?? 3000);

import { AppModule } from './app.module';

async function bootstrap() {
  try {
    const app = await NestFactory.create(AppModule);
    const port = process.env.PORT ? parseInt(process.env.PORT, 10) : 3000;
    await app.listen(port);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Failed to start application: ${message}`);
    process.exit(1);
  }

}
bootstrap();

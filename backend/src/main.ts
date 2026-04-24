import { NestFactory } from '@nestjs/core';
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

import { ValidationPipe } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';
import { DocumentBuilder, SwaggerModule } from '@nestjs/swagger';
import { AppModule } from './app.module';

async function bootstrap() {
  const app = await NestFactory.create(AppModule);

  app.useGlobalPipes(
    new ValidationPipe({
      whitelist: true,
      forbidNonWhitelisted: true,
      transform: true,
    }),
  );

  const config = new DocumentBuilder()
    .setTitle('Alien Gateway API')
    .setVersion('1.0')
    .setDescription(
      'Alien Gateway is a privacy-preserving username system for the Stellar network. ' +
      'It allows users to send and receive payments using human-readable identities like @username ' +
      'instead of long Stellar wallet addresses. Usernames are stored as zero-knowledge commitments, ' +
      'protecting user identity and wallet associations.',
    )
    .addTag('resolver', 'Username resolution and registration')
    .addTag('vault', 'Vault balance and payment management')
    .addTag('auction', 'Username auction system')
    .build();

  const document = SwaggerModule.createDocument(app, config);
  SwaggerModule.setup('api/docs', app, document);

  await app.listen(3000);
}
bootstrap();

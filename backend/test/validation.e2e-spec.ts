import { Test, TestingModule } from '@nestjs/testing';
import { INestApplication, ValidationPipe } from '@nestjs/common';
import * as request from 'supertest';
import { AppModule } from './../src/app.module';

describe('Global Validation (e2e)', () => {
  let app: INestApplication;

  beforeAll(async () => {
    const moduleFixture: TestingModule = await Test.createTestingModule({
      imports: [AppModule],
    }).compile();

    app = moduleFixture.createNestApplication();
    app.useGlobalPipes(
      new ValidationPipe({
        whitelist: true,
        forbidNonWhitelisted: true,
        transform: true,
      }),
    );
    await app.init();
  });

  afterAll(async () => {
    await app.close();
  });

  describe('Resolver Registration Validation', () => {
    it('should return 400 when username is too short', () => {
      return request(app.getHttpServer())
        .post('/resolver/register')
        .send({
          username: 'al',
          walletAddress: 'GABC1234STELLAR5678WALLETADDRESS',
          commitment: '0xabc1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcd',
        })
        .expect(400)
        .expect((res) => {
          expect(res.body.message).toContain('Username must be 3-20 characters long and contain only letters, numbers, and underscores');
        });
    });

    it('should return 400 when wallet address is invalid', () => {
      return request(app.getHttpServer())
        .post('/resolver/register')
        .send({
          username: 'alice',
          walletAddress: 'INVALID_ADDRESS',
          commitment: '0xabc1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcd',
        })
        .expect(400)
        .expect((res) => {
          expect(res.body.message).toContain('Invalid Stellar wallet address format');
        });
    });

    it('should return 400 when commitment is not a hex string', () => {
      return request(app.getHttpServer())
        .post('/resolver/register')
        .send({
          username: 'alice',
          walletAddress: 'GABC1234STELLAR5678WALLETADDRESS',
          commitment: 'not-a-hex-string',
        })
        .expect(400)
        .expect((res) => {
          expect(res.body.message).toContain('Commitment must be a 32-byte hex string prefixed with 0x');
        });
    });

    it('should return 400 when extra fields are provided (forbidNonWhitelisted)', () => {
      return request(app.getHttpServer())
        .post('/resolver/register')
        .send({
          username: 'alice',
          walletAddress: 'GABC1234STELLAR5678WALLETADDRESS',
          commitment: '0xabc1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcd',
          extraField: 'should-not-be-here',
        })
        .expect(400)
        .expect((res) => {
          expect(res.body.message).toContain('property extraField should not exist');
        });
    });
  });

  describe('Auction Bid Validation', () => {
    it('should return 400 when amount is not a number string', () => {
      return request(app.getHttpServer())
        .post('/auction/1/bid')
        .send({
          bidder: 'GABC1234STELLAR5678WALLETADDRESS',
          amount: 'not-a-number',
        })
        .expect(400)
        .expect((res) => {
          expect(res.body.message).toContain('amount must be a number string');
        });
    });
  });
});

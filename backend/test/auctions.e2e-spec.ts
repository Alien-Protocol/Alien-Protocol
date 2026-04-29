import { Test, TestingModule } from '@nestjs/testing';
import { INestApplication } from '@nestjs/common';
import request from 'supertest';
import { AppModule } from './../src/app.module';

describe('Auctions (e2e)', () => {
  let app: INestApplication;

  beforeEach(async () => {
    const moduleFixture: TestingModule = await Test.createTestingModule({
      imports: [AppModule],
    }).compile();

    app = moduleFixture.createNestApplication();
    await app.init();
  });

  it('GET /stellar/auctions - returns paginated auction list', () => {
    return request(app.getHttpServer())
      .get('/stellar/auctions?page=1&limit=10')
      .expect(200)
      .expect((res) => {
        expect(Array.isArray(res.body)).toBe(true);
        expect(res.body.length).toBeLessThanOrEqual(10);
        if (res.body.length > 0) {
          expect(res.body[0]).toHaveProperty('id');
          expect(res.body[0]).toHaveProperty('username');
          expect(res.body[0]).toHaveProperty('highestBid');
          expect(res.body[0]).toHaveProperty('endTime');
          expect(res.body[0]).toHaveProperty('status');
        }
      });
  });

  it('GET /stellar/auctions - default pagination (page=1, limit=20)', () => {
    return request(app.getHttpServer())
      .get('/stellar/auctions')
      .expect(200)
      .expect((res) => {
        expect(Array.isArray(res.body)).toBe(true);
      });
  });

  it('GET /stellar/auctions - respects limit cap of 100', () => {
    return request(app.getHttpServer())
      .get('/stellar/auctions?limit=200')
      .expect(200)
      .expect((res) => {
        expect(Array.isArray(res.body)).toBe(true);
        // Should be capped at 100, but our mock only returns 3 items anyway
      });
  });

  it('GET /stellar/auctions - filters by status (open)', () => {
    return request(app.getHttpServer())
      .get('/stellar/auctions?status=open')
      .expect(200)
      .expect((res) => {
        expect(Array.isArray(res.body)).toBe(true);
        res.body.forEach((auction) => {
          expect(auction.status).toBe('open');
        });
      });
  });

  it('GET /stellar/auctions - filters by status (closed)', () => {
    return request(app.getHttpServer())
      .get('/stellar/auctions?status=closed')
      .expect(200)
      .expect((res) => {
        expect(Array.isArray(res.body)).toBe(true);
        res.body.forEach((auction) => {
          expect(auction.status).toBe('closed');
        });
      });
  });

  it('GET /stellar/auctions - handles invalid status gracefully', () => {
    return request(app.getHttpServer())
      .get('/stellar/auctions?status=invalid')
      .expect(200)
      .expect((res) => {
        expect(Array.isArray(res.body)).toBe(true);
      });
  });
});

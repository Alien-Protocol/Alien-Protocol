import { Test, TestingModule } from '@nestjs/testing';
import { INestApplication } from '@nestjs/common';
import request from 'supertest';
import { AppModule } from './../src/app.module';

describe('Vaults (e2e)', () => {
  let app: INestApplication;

  beforeEach(async () => {
    const moduleFixture: TestingModule = await Test.createTestingModule({
      imports: [AppModule],
    }).compile();

    app = moduleFixture.createNestApplication();
    await app.init();
  });

  it('GET /vault/users/:id/vaults - returns vault list for existing user', () => {
    return request(app.getHttpServer())
      .get('/vault/users/GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN/vaults')
      .expect(200)
      .expect((res) => {
        expect(Array.isArray(res.body)).toBe(true);
        if (res.body.length > 0) {
          expect(res.body[0]).toHaveProperty('id');
          expect(res.body[0]).toHaveProperty('balance');
          expect(res.body[0]).toHaveProperty('status');
          expect(res.body[0]).toHaveProperty('createdAt');
        }
      });
  });

  it('GET /vault/users/:id/vaults - returns 404 for unknown user', () => {
    return request(app.getHttpServer())
      .get('/vault/users/GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG/vaults')
      .expect(404);
  });
});

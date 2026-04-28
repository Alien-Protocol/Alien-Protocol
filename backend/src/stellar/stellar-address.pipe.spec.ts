import { BadRequestException } from '@nestjs/common';
import { StellarAddressPipe } from './stellar-address.pipe';

describe('StellarAddressPipe', () => {
  let pipe: StellarAddressPipe;

  beforeEach(() => {
    pipe = new StellarAddressPipe();
  });

  // ── Valid addresses ──────────────────────────────────────────────────────────

  it('passes through a valid Stellar address unchanged', () => {
    const valid = 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN';
    expect(pipe.transform(valid)).toBe(valid);
  });

  it('accepts another valid address', () => {
    const valid = 'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGKUY5ZOBGNERCHLN4PEQX';
    expect(pipe.transform(valid)).toBe(valid);
  });

  // ── Invalid addresses — should throw 400 ────────────────────────────────────

  it('throws BadRequestException for an address not starting with G', () => {
    expect(() => pipe.transform('BAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN')).toThrow(
      BadRequestException,
    );
  });

  it('throws BadRequestException for an address that is too short', () => {
    expect(() => pipe.transform('GAAZI4TCR3')).toThrow(BadRequestException);
  });

  it('throws BadRequestException for an address that is too long', () => {
    expect(() => pipe.transform('GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNEXTRA')).toThrow(
      BadRequestException,
    );
  });

  it('throws BadRequestException for an empty string', () => {
    expect(() => pipe.transform('')).toThrow(BadRequestException);
  });

  it('throws BadRequestException for an Ethereum address', () => {
    expect(() => pipe.transform('0x71C7656EC7ab88b098defB751B7401B5f6d8976F')).toThrow(BadRequestException);
  });

  it('throws BadRequestException when address contains lowercase letters', () => {
    // Stellar addresses are uppercase Base32 — lowercase is invalid
    expect(() => pipe.transform('gaazi4tcr3ty5ojhctjc2a4qsy6cjwjh5iajtgkin2er7lbnvkoccwn')).toThrow(
      BadRequestException,
    );
  });

  it('error message includes the invalid value', () => {
    const bad = 'NOT_A_STELLAR_ADDRESS';
    try {
      pipe.transform(bad);
      fail('should have thrown');
    } catch (err) {
      expect(err).toBeInstanceOf(BadRequestException);
      expect((err as BadRequestException).message).toContain(bad);
    }
  });
});

import { PipeTransform, Injectable, BadRequestException } from '@nestjs/common';

/** Stellar public keys: always start with 'G' and are exactly 56 Base32 chars. */
const STELLAR_ADDRESS_RE = /^G[A-Z2-7]{55}$/;

@Injectable()
export class StellarAddressPipe implements PipeTransform<string, string> {
  transform(value: string): string {
    if (!STELLAR_ADDRESS_RE.test(value)) {
      throw new BadRequestException(
        `Invalid Stellar address: '${value}'. ` +
          'Expected a 56-character address starting with G (Base32).',
      );
    }
    return value;
  }
}

import { IsString, Matches } from 'class-validator';

export class VaultParamsDto {
  @IsString()
  @Matches(/^0x[a-fA-F0-9]{64}$/, {
    message: 'Commitment must be a 32-byte hex string prefixed with 0x',
  })
  commitment: string;
}

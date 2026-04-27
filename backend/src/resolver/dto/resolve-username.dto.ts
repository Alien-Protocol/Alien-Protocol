import { IsString, Length, Matches } from 'class-validator';

export class UsernameHashDto {
  @IsString()
  @Length(66, 66)
  @Matches(/^0x[0-9a-fA-F]{64}$/, {
    message: 'usernameHash must be a 0x-prefixed 32-byte hex string',
  })
  usernameHash!: string;
}

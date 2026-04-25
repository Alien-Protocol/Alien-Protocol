import { IsNumberString } from 'class-validator';

export class PaymentParamsDto {
  @IsNumberString()
  paymentId: string;
}

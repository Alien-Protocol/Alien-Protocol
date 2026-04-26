import { Controller, Get, Param, ParseIntPipe } from '@nestjs/common';
import { VaultService } from './vault.service';
import { VaultParamsDto } from './dto/vault-params.dto';
import { PaymentParamsDto } from './dto/payment-params.dto';

@Controller('vault')
export class VaultController {
  constructor(private readonly vaultService: VaultService) {}

  @Get(':commitment/balance')
  async getBalance(@Param() params: VaultParamsDto) {
    return this.vaultService.getBalance(params.commitment);
  }

  @Get(':commitment/payments')
  async getPayments(@Param() params: VaultParamsDto) {
    return this.vaultService.getPayments(params.commitment);
  }

  @Get('payment/:paymentId')
  async getPaymentById(@Param('paymentId', ParseIntPipe) paymentId: number) {
    return this.vaultService.getPaymentById(paymentId);
  }

  @Get(':commitment/autopay')
  async getAutoPay(@Param() params: VaultParamsDto) {
    return this.vaultService.getAutoPayRules(params.commitment);
  }

  @Get(':commitment/status')
  async getStatus(@Param() params: VaultParamsDto) {
    return this.vaultService.getStatus(params.commitment);
  }
}

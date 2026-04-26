import { Injectable, NotFoundException } from '@nestjs/common';
import { PrismaService } from '../prisma.service';
import { SorobanService } from '../soroban.service';

@Injectable()
export class VaultService {
  constructor(
    private readonly prisma: PrismaService,
    private readonly soroban: SorobanService,
  ) {}

  /**
   * Returns current balance and isActive status.
   * Hits DB cache first, falls back to contract move on cache miss.
   */
  async getBalance(commitment: string) {
    const vault = await this.prisma.vault.findUnique({
      where: { commitment },
    });

    if (!vault) {
      const balance = await this.soroban.getVaultBalance(commitment);
      if (balance === null) {
        throw new NotFoundException(`Vault ${commitment} not found`);
      }
      return { balance: balance.toString(), isActive: true };
    }

    return { balance: vault.balance, isActive: vault.isActive };
  }

  /**
   * Returns all scheduled payments for a commitment from the database.
   */
  async getPayments(commitment: string) {
    const payments = await this.prisma.scheduledPayment.findMany({
      where: {
        OR: [{ from: commitment }, { to: commitment }],
      },
      orderBy: { releaseAt: 'desc' },
    });

    return payments.map((p) => ({
      ...p,
      releaseAt: p.releaseAt.toString(),
    }));
  }

  /**
   * Returns single scheduled payment, falls back to contract call on cache miss.
   */
  async getPaymentById(paymentId: number) {
    const payment = await this.prisma.scheduledPayment.findUnique({
      where: { id: paymentId },
    });

    if (payment) {
      return {
        ...payment,
        releaseAt: payment.releaseAt.toString(),
      };
    }

    const contractPayment = await this.soroban.getScheduledPayment(paymentId);
    if (!contractPayment) {
      throw new NotFoundException(`Scheduled payment with ID ${paymentId} not found`);
    }
    return contractPayment;
  }

  /**
   * Returns all auto-pay rules for a commitment from the database.
   */
  async getAutoPayRules(commitment: string) {
    const rules = await this.prisma.autoPayRule.findMany({
      where: { from: commitment },
    });

    return rules.map((r) => ({
      ...r,
      interval: r.interval.toString(),
      lastPaid: r.lastPaid.toString(),
    }));
  }

  /**
   * Returns vault status: { exists: boolean, isActive: boolean | null }.
   * Disambiguates between non-existent and cancelled vaults.
   */
  async getStatus(commitment: string) {
    const vault = await this.prisma.vault.findUnique({
      where: { commitment },
    });

    if (vault) {
      return { exists: true, isActive: vault.isActive };
    }

    // Fallback to contract to check if it exists but hasn't been cached yet
    const isActive = await this.soroban.isVaultActive(commitment);
    return {
      exists: isActive !== null,
      isActive: isActive,
    };
  }
}

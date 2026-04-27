import { Injectable, NotFoundException } from '@nestjs/common';
import { PrismaService } from '../prisma.service';
import { SorobanService } from '../soroban.service';

@Injectable()
export class VaultService {
  /**
   * Initializes the VaultService with Prisma and Soroban services.
   * 
   * @param prisma - Service for database interactions.
   * @param soroban - Service for on-chain Soroban contract interactions.
   */
  constructor(
    private readonly prisma: PrismaService,
    private readonly soroban: SorobanService,
  ) {}

  /**
   * Retrieves the current balance and active status for a given vault commitment.
   * Checks the database cache first and falls back to a contract call if not found.
   * 
   * @param commitment - The unique vault commitment string.
   * @returns A promise that resolves to an object containing the balance and isActive status.
   * @throws {NotFoundException} If the vault is not found in the database or on-chain.
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
   * Retrieves all scheduled payments associated with a specific vault commitment.
   * 
   * @param commitment - The vault commitment string to filter payments by (either sender or receiver).
   * @returns A promise that resolves to an array of scheduled payments with formatted release dates.
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
   * Retrieves a specific scheduled payment by its unique identifier.
   * Checks the database cache first and falls back to a contract call if not found.
   * 
   * @param paymentId - The unique ID of the scheduled payment.
   * @returns A promise that resolves to the scheduled payment details.
   * @throws {NotFoundException} If the scheduled payment ID is not found.
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
   * Retrieves all auto-pay rules configured for a specific vault commitment.
   * 
   * @param commitment - The sender's vault commitment string.
   * @returns A promise that resolves to an array of auto-pay rules with formatted numeric fields.
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
   * Determines the existence and active status of a vault.
   * Disambiguates between non-existent and cancelled vaults by checking on-chain if necessary.
   * 
   * @param commitment - The unique vault commitment string.
   * @returns A promise that resolves to an object indicating if the vault exists and its active status.
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

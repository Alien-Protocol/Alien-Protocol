/**
 * TypeScript interfaces for Stellar contract return types
 */

export enum ChainType {
  Evm = 'Evm',
  Bitcoin = 'Bitcoin',
  Solana = 'Solana',
  Cosmos = 'Cosmos',
}

export enum PrivacyMode {
  Normal = 'Normal',
  Shielded = 'Shielded',
}

export interface ScheduledPayment {
  from: string;
  to: string;
  token: string;
  amount: string;
  release_at: number;
  executed: boolean;
}

export interface VaultConfig {
  owner: string;
  token: string;
  created_at: number;
}

export interface VaultState {
  balance: string;
  is_active: boolean;
}

export interface ResolveData {
  wallet: string;
  memo?: number;
}

// Contract method return types
export type GetOwnerResult = string | null;
export type ResolveStellarResult = string;
export type GetChainAddressResult = string | null;
export type GetVaultBalanceResult = string | null;
export type GetScheduledPaymentResult = ScheduledPayment | null;
export type IsVaultActiveResult = boolean | null;
export type GetCreatedAtResult = number | null;
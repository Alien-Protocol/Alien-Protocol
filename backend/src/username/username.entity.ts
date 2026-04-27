import { Column, Entity, PrimaryColumn } from 'typeorm';

@Entity({ name: 'usernames' })
export class Username {
  @PrimaryColumn({ name: 'username_hash', type: 'varchar', length: 66 })
  usernameHash!: string;

  @Column({ name: 'stellar_address', type: 'varchar', nullable: true })
  stellarAddress!: string | null;

  @Column({ type: 'varchar', nullable: true })
  owner!: string | null;

  @Column({ name: 'evm_address', type: 'varchar', nullable: true })
  evmAddress!: string | null;

  @Column({ name: 'bitcoin_address', type: 'varchar', nullable: true })
  bitcoinAddress!: string | null;

  @Column({ name: 'solana_address', type: 'varchar', nullable: true })
  solanaAddress!: string | null;

  @Column({ name: 'cosmos_address', type: 'varchar', nullable: true })
  cosmosAddress!: string | null;

  @Column({ name: 'registered_at', type: 'timestamptz', nullable: true })
  registeredAt!: Date | null;
}

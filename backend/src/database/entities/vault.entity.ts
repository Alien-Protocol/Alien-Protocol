import { Entity, Column, PrimaryColumn } from 'typeorm';

@Entity('vaults')
export class Vault {
  @PrimaryColumn()
  commitment: string;

  @Column()
  owner: string;

  @Column()
  token: string;

  @Column({ type: 'bigint', default: '0' })
  balance: string;

  @Column({ default: true })
  isActive: boolean;

  @Column({ type: 'bigint' })
  createdAt: string;
}

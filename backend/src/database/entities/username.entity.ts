import { Entity, Column, PrimaryColumn } from 'typeorm';

@Entity('usernames')
export class Username {
  @PrimaryColumn()
  hash: string;

  @Column()
  owner: string;

  @Column()
  stellarAddress: string;

  @Column({ type: 'jsonb', default: {} })
  chainAddresses: Record<string, string>;

  @Column({ type: 'bigint' })
  registeredAt: string;

  @Column({ type: 'bigint' })
  updatedAt: string;
}

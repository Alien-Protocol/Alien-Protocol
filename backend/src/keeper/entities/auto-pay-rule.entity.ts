import { Entity, PrimaryColumn, Column, Index } from 'typeorm';

@Entity()
@Index(['isActive', 'lastPaid', 'interval'])
export class AutoPayRule {
  @PrimaryColumn()
  ruleId: number;

  @Column()
  fromCommitment: string;

  @Column()
  toCommitment: string;

  @Column()
  token: string;

  @Column({ type: 'bigint' })
  amount: string;

  @Column({ type: 'bigint' })
  interval: string;

  @Column({ type: 'bigint', default: '0' })
  lastPaid: string;

  @Column({ default: true })
  isActive: boolean;

  @Column({ default: false })
  needsAttention: boolean;
}

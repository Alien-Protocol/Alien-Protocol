import { Entity, Column, PrimaryColumn } from 'typeorm';

@Entity('auto_pay_rules')
export class AutoPayRule {
  @PrimaryColumn()
  ruleId: string;

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
}

import { Entity, Column, PrimaryColumn } from 'typeorm';

@Entity('payments')
export class Payment {
  @PrimaryColumn()
  paymentId: string;

  @Column()
  fromCommitment: string;

  @Column()
  toCommitment: string;

  @Column({ type: 'bigint' })
  amount: string;

  @Column({ type: 'bigint' })
  releaseAt: string;

  @Column({ default: false })
  executed: boolean;

  @Column()
  token: string;
}

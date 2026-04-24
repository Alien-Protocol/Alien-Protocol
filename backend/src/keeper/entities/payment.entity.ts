import { Entity, PrimaryGeneratedColumn, Column, Index, Unique } from 'typeorm';

@Entity()
@Index(['executed', 'releaseAt'])
export class Payment {
  @PrimaryGeneratedColumn()
  id: number;

  @Column({ unique: true })
  paymentId: number;

  @Column({ default: false })
  executed: boolean;

  @Column({ type: 'datetime' })
  releaseAt: Date;
}

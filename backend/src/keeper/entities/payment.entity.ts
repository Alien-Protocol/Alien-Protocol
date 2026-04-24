import { Entity, PrimaryGeneratedColumn, Column } from 'typeorm';

@Entity()
export class Payment {
  @PrimaryGeneratedColumn()
  id: number;

  @Column()
  paymentId: number;

  @Column({ default: false })
  executed: boolean;

  @Column({ type: 'datetime' })
  releaseAt: Date;
}

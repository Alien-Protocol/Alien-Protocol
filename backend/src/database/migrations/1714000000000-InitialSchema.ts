import { MigrationInterface, QueryRunner, Table, TableIndex } from "typeorm";

export class InitialSchema1714000000000 implements MigrationInterface {
    name = 'InitialSchema1714000000000'

    public async up(queryRunner: QueryRunner): Promise<void> {
        await queryRunner.createTable(new Table({
            name: "usernames",
            columns: [
                { name: "hash", type: "varchar", isPrimary: true },
                { name: "owner", type: "varchar" },
                { name: "stellarAddress", type: "varchar" },
                { name: "chainAddresses", type: "jsonb", default: "'{}'" },
                { name: "registeredAt", type: "bigint" },
                { name: "updatedAt", type: "bigint" }
            ]
        }), true);

        await queryRunner.createTable(new Table({
            name: "vaults",
            columns: [
                { name: "commitment", type: "varchar", isPrimary: true },
                { name: "owner", type: "varchar" },
                { name: "token", type: "varchar" },
                { name: "balance", type: "bigint", default: "'0'" },
                { name: "isActive", type: "boolean", default: true },
                { name: "createdAt", type: "bigint" }
            ]
        }), true);

        await queryRunner.createTable(new Table({
            name: "payments",
            columns: [
                { name: "paymentId", type: "varchar", isPrimary: true },
                { name: "fromCommitment", type: "varchar" },
                { name: "toCommitment", type: "varchar" },
                { name: "amount", type: "bigint" },
                { name: "releaseAt", type: "bigint" },
                { name: "executed", type: "boolean", default: false },
                { name: "token", type: "varchar" }
            ]
        }), true);

        await queryRunner.createTable(new Table({
            name: "auto_pay_rules",
            columns: [
                { name: "ruleId", type: "varchar", isPrimary: true },
                { name: "fromCommitment", type: "varchar" },
                { name: "toCommitment", type: "varchar" },
                { name: "token", type: "varchar" },
                { name: "amount", type: "bigint" },
                { name: "interval", type: "bigint" },
                { name: "lastPaid", type: "bigint", default: "'0'" },
                { name: "isActive", type: "boolean", default: true },
                { name: "needsAttention", type: "boolean", default: false }
            ]
        }), true);

        await queryRunner.createIndex("auto_pay_rules", new TableIndex({
            name: "IDX_auto_pay_rules_isActive_lastPaid_interval",
            columnNames: ["isActive", "lastPaid", "interval"],
        }));
    }

    public async down(queryRunner: QueryRunner): Promise<void> {
        await queryRunner.dropIndex("auto_pay_rules", "IDX_auto_pay_rules_isActive_lastPaid_interval");
        await queryRunner.dropTable("auto_pay_rules");
        await queryRunner.dropTable("payments");
        await queryRunner.dropTable("vaults");
        await queryRunner.dropTable("usernames");
    }
}

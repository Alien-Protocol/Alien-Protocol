"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.InitialSchema1714000000000 = void 0;
const typeorm_1 = require("typeorm");
class InitialSchema1714000000000 {
    name = 'InitialSchema1714000000000';
    async up(queryRunner) {
        await queryRunner.createTable(new typeorm_1.Table({
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
        await queryRunner.createTable(new typeorm_1.Table({
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
        await queryRunner.createTable(new typeorm_1.Table({
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
        await queryRunner.createTable(new typeorm_1.Table({
            name: "auto_pay_rules",
            columns: [
                { name: "ruleId", type: "varchar", isPrimary: true },
                { name: "fromCommitment", type: "varchar" },
                { name: "toCommitment", type: "varchar" },
                { name: "token", type: "varchar" },
                { name: "amount", type: "bigint" },
                { name: "interval", type: "bigint" },
                { name: "lastPaid", type: "bigint", default: "'0'" },
                { name: "isActive", type: "boolean", default: true }
            ]
        }), true);
    }
    async down(queryRunner) {
        await queryRunner.dropTable("auto_pay_rules");
        await queryRunner.dropTable("payments");
        await queryRunner.dropTable("vaults");
        await queryRunner.dropTable("usernames");
    }
}
exports.InitialSchema1714000000000 = InitialSchema1714000000000;
//# sourceMappingURL=1714000000000-InitialSchema.js.map
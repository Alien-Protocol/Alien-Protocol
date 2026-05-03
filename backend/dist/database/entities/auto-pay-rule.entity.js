"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
var __metadata = (this && this.__metadata) || function (k, v) {
    if (typeof Reflect === "object" && typeof Reflect.metadata === "function") return Reflect.metadata(k, v);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.AutoPayRule = void 0;
const typeorm_1 = require("typeorm");
let AutoPayRule = class AutoPayRule {
};
exports.AutoPayRule = AutoPayRule;
__decorate([
    (0, typeorm_1.PrimaryColumn)(),
    __metadata("design:type", String)
], AutoPayRule.prototype, "ruleId", void 0);
__decorate([
    (0, typeorm_1.Column)(),
    __metadata("design:type", String)
], AutoPayRule.prototype, "fromCommitment", void 0);
__decorate([
    (0, typeorm_1.Column)(),
    __metadata("design:type", String)
], AutoPayRule.prototype, "toCommitment", void 0);
__decorate([
    (0, typeorm_1.Column)(),
    __metadata("design:type", String)
], AutoPayRule.prototype, "token", void 0);
__decorate([
    (0, typeorm_1.Column)({ type: 'bigint' }),
    __metadata("design:type", String)
], AutoPayRule.prototype, "amount", void 0);
__decorate([
    (0, typeorm_1.Column)({ type: 'bigint' }),
    __metadata("design:type", String)
], AutoPayRule.prototype, "interval", void 0);
__decorate([
    (0, typeorm_1.Column)({ type: 'bigint', default: '0' }),
    __metadata("design:type", String)
], AutoPayRule.prototype, "lastPaid", void 0);
__decorate([
    (0, typeorm_1.Column)({ default: true }),
    __metadata("design:type", Boolean)
], AutoPayRule.prototype, "isActive", void 0);
exports.AutoPayRule = AutoPayRule = __decorate([
    (0, typeorm_1.Entity)('auto_pay_rules')
], AutoPayRule);
//# sourceMappingURL=auto-pay-rule.entity.js.map
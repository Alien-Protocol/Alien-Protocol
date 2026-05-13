
# 🌉 Alien Gateway

[![Smart Contracts CI](https://github.com/Alien-Protocol/Alien-Gateway/actions/workflows/contract.yml/badge.svg)](https://github.com/Alien-Protocol/Alien-Gateway/actions/workflows/contract.yml)
[![Validator CI](https://github.com/Alien-Protocol/Alien-Gateway/actions/workflows/pre-commit-validation.yml/badge.svg)](https://github.com/Alien-Protocol/Alien-Gateway/actions/workflows/pre-commit-validation.yml)


<p align="center">
  <img 
    src="./assets/alien_protocol_banner.gif" 
    alt="Alien Protocol"
    width="100%"
    style="max-width:900px; border-radius:20px; box-shadow:0 0 40px rgba(0,255,170,.35);"
  />
</p>

> Send crypto to `@username` instead of a long wallet address — built for Stellar.

Alien Gateway is a **privacy-preserving username system for the Stellar network**.  
It allows users to send and receive payments using **human-readable identities** like `@username` instead of long Stellar wallet addresses.

Unlike traditional naming systems, usernames are **never stored on-chain in plaintext**.  
They are stored as **zero-knowledge commitments**, protecting user identity and wallet associations.

---

## ✨ Features

- Send payments using `@username`
- Human-readable Stellar identities
- Privacy-preserving wallet linking
- Optional private payment routing
- Developer-friendly SDK for integration

---

## 🧠 How It Works

1. User registers a `@username`
2. Username is stored as a **ZK commitment**
3. The system verifies uniqueness using **zero-knowledge proofs**
4. The username resolves to a linked **Stellar wallet**
5. Payments can be sent directly using `@username`

---

## ⚙️ Tech Stack

- **Smart Contracts:** Rust + Soroban  
- **ZK Circuits:** Circom  
- **Proof System:** Groth16  
- **Hash Function:** Poseidon  
- **SDK:** TypeScript  

---

## 🚀 Vision

**One username. One identity. Built for Stellar.**

Alien Gateway aims to become the **identity and payment resolution layer for the Stellar ecosystem**.

## 🌐 Connect with us

<p align="center">
  <a href="https://t.me/alien_protocol_xyz">
    <img src="https://img.shields.io/badge/Telegram-Join%20Alien%20Protocol-2CA5E0?logo=telegram&logoColor=white&style=for-the-badge"/>
  </a>
</p>

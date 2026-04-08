# Freeproxy Clear Pools Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a destructive action in Settings that clears both the gateway freeproxy pool and the register proxy pool.

**Architecture:** Expose a new gateway RPC `gateway/freeProxy/clear`, return a compact summary payload, and wire it to a confirmed destructive button in the existing freeproxy quick-sync card.

**Tech Stack:** Rust service RPC, Next.js App Router, TypeScript strict mode, shadcn/ui

---

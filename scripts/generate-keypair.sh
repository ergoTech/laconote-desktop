#!/usr/bin/env bash
set -euo pipefail

echo "Generating Tauri Ed25519 signing keypair..."
echo "This key pair is used to sign update artifacts for the auto-updater."
echo ""

npx @tauri-apps/cli@latest signer generate -w ./tauri-signing.key

echo ""
echo "Done! Files created:"
echo "  tauri-signing.key        — PRIVATE key (keep secret, add to GitHub Secrets as TAURI_SIGNING_PRIVATE_KEY)"
echo "  tauri-signing.key.pub    — PUBLIC key (add to tauri.conf.json > plugins > updater > pubkey)"
echo ""
echo "Next steps:"
echo "  1. Copy the public key from tauri-signing.key.pub"
echo "  2. Paste it into src-tauri/tauri.conf.json under plugins.updater.pubkey"
echo "  3. Add TAURI_SIGNING_PRIVATE_KEY to GitHub repository secrets"
echo "  4. Add TAURI_SIGNING_PRIVATE_KEY_PASSWORD to GitHub repository secrets (if key has a password)"
echo "  5. Delete tauri-signing.key from disk — store it only in GitHub Secrets"

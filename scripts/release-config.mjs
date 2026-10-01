#!/usr/bin/env node
// The Tauri config a release build merges over tauri.conf.json, printed as
// JSON: the version from the tag, updater artifacts when the signing key is
// there, and Windows Authenticode when a certificate was imported.
//
//   node scripts/release-config.mjs <version>
const version = process.argv[2] ?? "";
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`release-config: "${version}" is not a version (tag v1.2.3 or v1.2.3-beta.1)`);
  process.exit(1);
}
const config = { version, bundle: { createUpdaterArtifacts: Boolean(process.env.TAURI_SIGNING_PRIVATE_KEY) } };
const thumbprint = process.env.WINDOWS_THUMBPRINT;
if (thumbprint) {
  config.bundle.windows = {
    certificateThumbprint: thumbprint,
    digestAlgorithm: "sha256",
    timestampUrl: "http://timestamp.digicert.com",
  };
}
// The public key in the app is the build variable; the manifest's own
// `plugins.updater.pubkey` stays empty in the tree and is filled here so the
// updater's config check sees the same key.
if (process.env.GEEK_UPDATER_PUBKEY) {
  config.plugins = { updater: { pubkey: process.env.GEEK_UPDATER_PUBKEY } };
}
process.stdout.write(JSON.stringify(config, null, 2) + "\n");

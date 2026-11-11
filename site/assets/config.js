// Site configuration.
window.POSTPLANT = {
  REPO: "jeelnadaa/postplant",
  // Asset name patterns produced by the release workflow (plan.md section 20.1).
  ASSETS: {
    installer: /_x64-setup\.exe$/i,
    offline: /_x64-offline-setup\.exe$/i,
    portable: /_x64-portable\.zip$/i,
    checksums: /^SHA256SUMS\.txt$/i,
  },
};

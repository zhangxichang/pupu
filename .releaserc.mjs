export default {
  branches:
    process.env.PRERELEASE === "true"
      ? [{ name: "main", prerelease: true }]
      : ["main"],
  plugins: [
    ["@semantic-release/commit-analyzer", { preset: "conventionalcommits" }],
    [
      "@semantic-release/release-notes-generator",
      { preset: "conventionalcommits" },
    ],
    [
      "@semantic-release/exec",
      { verifyReleaseCmd: "echo '${nextRelease.version}' > version" },
    ],
    ["@semantic-release/github", { assets: "artifacts/*" }],
  ],
};

export default {
  branches: [{ name: "main", prerelease: process.env.PRERELEASE === "true" }],
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
    ["@semantic-release/github", { assets: "artifact/*" }],
  ],
};

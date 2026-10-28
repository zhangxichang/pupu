const CHANNELS = ["alpha", "beta", "stable"];
const channel = process.env.PRERELEASE;

if (!CHANNELS.includes(channel)) {
  throw new Error(
    `PRERELEASE 必须是 ${CHANNELS.join(" / ")} 之一，当前值：${channel ?? "<未设置>"}`,
  );
}

const prerelease = channel === "stable" ? false : channel;

export default {
  branches: [
    "placeholder",
    {
      name: "main",
      prerelease,
    },
  ],
  plugins: [
    ["@semantic-release/commit-analyzer", { preset: "conventionalcommits" }],
    ["@semantic-release/release-notes-generator", { preset: "conventionalcommits" }],
    [
      "@semantic-release/exec",
      {
        verifyReleaseCmd: "echo ${nextRelease.version} > version",
      },
    ],
    ["@semantic-release/github", { assets: ["artifacts/*"] }],
  ],
};

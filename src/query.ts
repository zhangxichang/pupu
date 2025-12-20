import { query } from "@solidjs/router";
import { Octokit } from "octokit";

export const get_version = query(async () => {
  const version = await fetch("/version");
  const content_type = version.headers.get("Content-Type");
  if (content_type !== "text/html") {
    return await version.text();
  }
}, "version");

export const get_contributors = query(async () => {
  const contributors = await new Octokit().rest.repos.listContributors({
    owner: "ZhangXiChang",
    repo: "starlink",
  });
  return contributors.data;
}, "contributors");

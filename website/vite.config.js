import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const repository = process.env.GITHUB_REPOSITORY ?? "";
const repositoryName = repository.split("/")[1] ?? "";
const isGitHubActions = process.env.GITHUB_ACTIONS === "true";
const basePath = isGitHubActions && repositoryName ? `/${repositoryName}/` : "/";

export default defineConfig({
  base: basePath,
  plugins: [vue()],
});

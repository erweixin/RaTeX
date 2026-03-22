/** GitHub repo root (no trailing slash). */
export const REPO = "https://github.com/erweixin/RaTeX";

/** Link to a file or directory on the default branch. */
export function repoBlob(path: string): string {
  return `${REPO}/blob/main/${path.replace(/^\//, "")}`;
}

/**
 * Injects KaTeX font stylesheet. Paths differ when pages live under /website/
 * (repo root server) vs site root (GitHub Pages / local _site).
 *
 * Use an absolute URL when not under /website/ so /RaTeX (no trailing slash) still resolves under base.
 */
(function () {
  function getSiteDirUrl() {
    var u = new URL(location.href);
    var path = u.pathname;
    if (!path.endsWith("/")) {
      var last = path.split("/").pop() || "";
      if (last.indexOf(".") !== -1) {
        path = path.replace(/\/[^/]+$/, "/");
      } else {
        path = path + "/";
      }
    }
    u.pathname = path || "/";
    return u;
  }
  var g = typeof globalThis !== "undefined" ? globalThis : window;
  var path = location.pathname;
  var href;
  if (typeof g.__RATEX_SITE_BASE__ === "string" && g.__RATEX_SITE_BASE__.length > 0) {
    var base = g.__RATEX_SITE_BASE__;
    if (!base.endsWith("/")) base += "/";
    href = new URL("platforms/web/fonts.css", new URL(base, location.origin)).href;
  } else if (path.indexOf("/website/") !== -1) {
    href = "../platforms/web/fonts.css";
  } else {
    href = new URL("platforms/web/fonts.css", getSiteDirUrl()).href;
  }
  var link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = href;
  document.head.insertBefore(link, document.head.firstChild);
})();

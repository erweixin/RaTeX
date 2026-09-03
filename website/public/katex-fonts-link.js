/**
 * Injects the KaTeX font stylesheet and publishes a promise that resolves only
 * after its @font-face rules are available. Same URL rules as gallery.js.
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
  var path = location.pathname || "";
  var href;
  if (typeof g.__RATEX_SITE_BASE__ === "string" && g.__RATEX_SITE_BASE__.length > 0) {
    var base = g.__RATEX_SITE_BASE__;
    if (!base.endsWith("/")) base += "/";
    href = new URL("platforms/web/fonts.css", new URL(base, location.origin)).href;
  } else if (path.indexOf("/website/") !== -1) {
    href = new URL("../platforms/web/fonts.css", location.href).href;
  } else if (path.startsWith("/RaTeX/") || path === "/RaTeX") {
    href = new URL("platforms/web/fonts.css", new URL("/RaTeX/", location.origin)).href;
  } else if (location.protocol === "file:") {
    href = new URL("platforms/web/fonts.css", getSiteDirUrl()).href;
  } else if (/^\/demo(\/|$)/.test(path) || /^\/zh(\/|$)/.test(path)) {
    href = new URL("/platforms/web/fonts.css", location.origin).href;
  } else {
    href = new URL("platforms/web/fonts.css", getSiteDirUrl()).href;
  }
  var link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = href;
  link.setAttribute("data-ratex-fonts-stylesheet", "");

  g.__RATEX_FONTS_STYLESHEET_READY__ = new Promise(function (resolve) {
    link.addEventListener(
      "load",
      function () {
        resolve({ ok: true, href: href });
      },
      { once: true }
    );
    link.addEventListener(
      "error",
      function () {
        resolve({ ok: false, href: href });
      },
      { once: true }
    );
  });

  document.head.insertBefore(link, document.head.firstChild);
})();

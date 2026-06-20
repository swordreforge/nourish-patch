/* Nourish site — progressive enhancement only.
   The page is fully readable with this file deleted. */
(function () {
  "use strict";

  // Mark that JS is running; CSS only hides .reveal elements under .js
  document.documentElement.classList.add("js");

  var reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* ---------- Starfield with 3 parallax depths (hero only) ---------- */
  var sky = document.getElementById("stars");
  if (sky) {
    var LAYERS = [
      { count: 36, size: [1, 1.6], speed: 0.05, op: [0.15, 0.45] }, // far
      { count: 24, size: [1.6, 2.4], speed: 0.12, op: [0.25, 0.7] }, // mid
      { count: 14, size: [2.4, 3.4], speed: 0.22, op: [0.35, 0.95] } // near
    ];
    var layerEls = LAYERS.map(function (cfg) {
      var layer = document.createElement("div");
      layer.style.position = "absolute";
      layer.style.inset = "0";
      layer.dataset.speed = cfg.speed;
      for (var i = 0; i < cfg.count; i++) {
        var s = document.createElement("span");
        s.className = "star";
        var size = cfg.size[0] + Math.random() * (cfg.size[1] - cfg.size[0]);
        s.style.width = s.style.height = size.toFixed(1) + "px";
        s.style.left = (Math.random() * 100).toFixed(2) + "%";
        s.style.top = (Math.random() * 100).toFixed(2) + "%";
        s.style.setProperty("--o1", cfg.op[0].toFixed(2));
        s.style.setProperty("--o2", cfg.op[1].toFixed(2));
        s.style.setProperty("--tw", (3 + Math.random() * 5).toFixed(1) + "s");
        layer.appendChild(s);
      }
      sky.appendChild(layer);
      return layer;
    });

    // Parallax: decoration only, transform-only, rAF-throttled
    if (!reduceMotion) {
      var ticking = false;
      var onScroll = function () {
        if (ticking) return;
        ticking = true;
        requestAnimationFrame(function () {
          var y = window.scrollY;
          layerEls.forEach(function (layer) {
            layer.style.transform =
              "translateY(" + (y * parseFloat(layer.dataset.speed)).toFixed(1) + "px)";
          });
          ticking = false;
        });
      };
      window.addEventListener("scroll", onScroll, { passive: true });
    }
  }

  /* ---------- Scroll reveals (once per element) ---------- */
  var reveals = document.querySelectorAll(".reveal");
  if (reduceMotion || !("IntersectionObserver" in window)) {
    reveals.forEach(function (el) { el.classList.add("in"); });
  } else {
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (e.isIntersecting) {
          e.target.classList.add("in");
          io.unobserve(e.target);
        }
      });
    }, { threshold: 0.12, rootMargin: "0px 0px -8% 0px" });
    reveals.forEach(function (el) { io.observe(el); });
  }

  /* ---------- Copy buttons on code blocks ---------- */
  document.querySelectorAll("[data-copy]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var code = btn.parentElement.querySelector("code");
      if (!code || !navigator.clipboard) return;
      navigator.clipboard.writeText(code.textContent).then(function () {
        var old = btn.textContent;
        btn.textContent = "Copied";
        setTimeout(function () { btn.textContent = old; }, 1600);
      });
    });
  });

  /* ---------- Feature loop videos: play only while visible ----------
     Activates automatically once placeholders are replaced by
     <video muted loop playsinline> elements. */
  var vids = document.querySelectorAll("video[loop]");
  if (vids.length && "IntersectionObserver" in window && !reduceMotion) {
    var vio = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (e.isIntersecting) { e.target.play().catch(function () {}); }
        else { e.target.pause(); }
      });
    }, { threshold: 0.35 });
    vids.forEach(function (v) { vio.observe(v); });
  }
})();

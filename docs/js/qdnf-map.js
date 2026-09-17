// QDNF map — honeycomb tap/keyboard disclosure. No Host. No framework.
(function () {
  function bindHoneycomb(root) {
    const panel = root.querySelector('#hex-detail');
    if (!panel) return;
    const nameEl = panel.querySelector('[data-hex-name]');
    const plainEl = panel.querySelector('[data-hex-plain]');
    const techEl = panel.querySelector('[data-hex-tech]');
    const triggers = root.querySelectorAll('[data-hex]');

    function close() {
      triggers.forEach(function (el) {
        el.setAttribute('aria-expanded', 'false');
      });
      panel.hidden = true;
    }

    function open(btn) {
      const id = btn.getAttribute('data-hex');
      const already = btn.getAttribute('aria-expanded') === 'true';
      close();
      if (already) return;
      root.querySelectorAll('[data-hex="' + id + '"]').forEach(function (el) {
        el.setAttribute('aria-expanded', 'true');
      });
      if (nameEl) nameEl.textContent = btn.getAttribute('data-name') || '';
      if (plainEl) plainEl.textContent = btn.getAttribute('data-plain') || '';
      if (techEl) techEl.textContent = btn.getAttribute('data-tech') || '';
      panel.hidden = false;
    }

    triggers.forEach(function (el) {
      el.addEventListener('click', function () {
        open(el);
      });
    });

    root.addEventListener('keydown', function (event) {
      if (event.key === 'Escape') close();
    });
  }

  if (typeof document === 'undefined') return;
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', function () {
      bindHoneycomb(document);
    });
  } else {
    bindHoneycomb(document);
  }
})();

// Shared Pages markdown reader. Fetches local .md under docs/ and renders it
// with the 0.0.38 reading chrome. Never treats a locator as a human.

(function () {
  const ALLOWED_PREFIXES = [
    'manuals/',
    'standards/',
    'work-in-progress/',
    'releases/',
    'plans/',
  ];

  function docsRootFromScript() {
    const script = document.querySelector('script[src*="markdown-doc.js"]');
    let root = '';
    if (script) {
      const src = script.getAttribute('src') || '';
      if (src.startsWith('http') || src.startsWith('/')) {
        root = src.replace(/js\/markdown-doc\.js.*$/, '');
      } else {
        root = src.replace(/js\/markdown-doc\.js.*$/, '') || '';
      }
    }
    const pagesBase = window.location.pathname.match(/^(.*\/qualiaDB\/)/);
    if (pagesBase) {
      const base = pagesBase[1];
      if (!root || (!root.startsWith('/') && !root.startsWith('http'))) {
        return base;
      }
    }
    return root;
  }

  function stripKnownPrefix(path) {
    return path
      .replace(/^\/+/, '')
      .replace(/^qualiaDB\//, '')
      .replace(/^docs\//, '');
  }

  function normalizeDocPath(raw) {
    if (!raw) return null;
    let path = String(raw).trim();
    const hashIndex = path.indexOf('#');
    if (hashIndex >= 0) path = path.slice(0, hashIndex);
    try {
      path = decodeURIComponent(path);
    } catch (_) {
      return null;
    }
    path = stripKnownPrefix(path.replace(/\\/g, '/'));
    if (!path || path.includes('..') || path.startsWith('/') || /[:?]/.test(path)) {
      return null;
    }
    if (!path.toLowerCase().endsWith('.md')) return null;
    if (!ALLOWED_PREFIXES.some((prefix) => path.startsWith(prefix))) return null;
    return path;
  }

  function parentDir(docPath) {
    const idx = docPath.lastIndexOf('/');
    return idx === -1 ? '' : docPath.slice(0, idx + 1);
  }

  function resolveRelativeDoc(currentDoc, href) {
    const [pathPart, hash] = href.split('#');
    if (!pathPart) return { path: currentDoc, hash: hash || '' };
    let joined = pathPart;
    if (!pathPart.startsWith('/')) {
      joined = parentDir(currentDoc) + pathPart;
    } else {
      joined = stripKnownPrefix(pathPart);
    }
    const parts = [];
    for (const part of joined.split('/')) {
      if (!part || part === '.') continue;
      if (part === '..') {
        if (!parts.length) return null;
        parts.pop();
        continue;
      }
      parts.push(part);
    }
    const path = parts.join('/');
    const normalized = normalizeDocPath(path);
    if (!normalized) return null;
    return { path: normalized, hash: hash || '' };
  }

  function readerHref(docPath, hash) {
    const root = docsRootFromScript();
    const url = `${root}read.html?doc=${encodeURIComponent(docPath)}`;
    return hash ? `${url}#${hash}` : url;
  }

  function rewriteMarkdownLinks(rootEl, currentDoc) {
    rootEl.querySelectorAll('a[href]').forEach((anchor) => {
      const href = anchor.getAttribute('href');
      if (!href || href.startsWith('http') || href.startsWith('mailto:') || href.startsWith('#')) {
        return;
      }
      const clean = href.split('?')[0];
      if (!/\.md(?:#|$)/i.test(clean) && !clean.toLowerCase().endsWith('.md')) return;
      const resolved = resolveRelativeDoc(currentDoc, href);
      if (!resolved) return;
      anchor.setAttribute('href', readerHref(resolved.path, resolved.hash));
    });
  }

  function inferCurrentDocFromLocation() {
    let path = window.location.pathname;
    const match = path.match(/\/qualiaDB\/(.*)$/);
    let rel = match ? match[1] : path.replace(/^\//, '');
    rel = stripKnownPrefix(rel);
    if (!rel || rel === 'read.html') return null;
    if (rel.endsWith('/')) return normalizeDocPath(`${rel}README.md`);
    if (rel.endsWith('.html')) {
      return normalizeDocPath(rel.replace(/\.html$/, '.md'));
    }
    return normalizeDocPath(rel);
  }

  function headingId(text, used) {
    const base = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .trim()
      .replace(/\s+/g, '-')
      .slice(0, 72) || 'section';
    let id = base;
    let n = 2;
    while (used.has(id)) {
      id = `${base}-${n}`;
      n += 1;
    }
    used.add(id);
    return id;
  }

  function buildToc(contentEl, tocEl) {
    if (!tocEl) return;
    tocEl.innerHTML = '<p class="q-kicker" style="margin:0 0 0.6rem">Contents</p>';
    const used = new Set();
    const headings = contentEl.querySelectorAll('h2, h3');
    headings.forEach((heading) => {
      if (!heading.id) heading.id = headingId(heading.textContent || 'section', used);
      const link = document.createElement('a');
      link.href = `#${heading.id}`;
      link.textContent = heading.textContent || heading.id;
      if (heading.tagName === 'H3') link.style.paddingLeft = '1.1rem';
      tocEl.appendChild(link);
    });

    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        tocEl.querySelectorAll('a').forEach((anchor) => {
          anchor.classList.toggle('active', anchor.getAttribute('href') === `#${entry.target.id}`);
        });
      });
    }, { rootMargin: '-80px 0px -70% 0px' });
    headings.forEach((heading) => observer.observe(heading));
  }

  function sourceHref(docPath) {
    return `https://github.com/mediaprophet/qualiaDB/blob/0.0.38/docs/${docPath}`;
  }

  async function renderReader() {
    const params = new URLSearchParams(window.location.search);
    const requested = params.get('doc');
    const docPath = normalizeDocPath(requested);
    const titleEl = document.getElementById('doc-title');
    const crumbEl = document.getElementById('doc-crumb');
    const contentEl = document.getElementById('doc-content');
    const tocEl = document.getElementById('doc-toc');
    const sourceEl = document.getElementById('doc-source');
    if (!contentEl) return;

    if (!docPath) {
      contentEl.innerHTML = '<p>That path is not a readable Pages document. Stay inside manuals, standards, work-in-progress, releases, or plans.</p>';
      if (titleEl) titleEl.textContent = 'Document not available';
      return;
    }

    if (crumbEl) crumbEl.textContent = docPath;
    if (sourceEl) {
      sourceEl.href = sourceHref(docPath);
      sourceEl.hidden = false;
    }

    try {
      const root = docsRootFromScript();
      const response = await fetch(`${root}${docPath}`, { cache: 'no-cache' });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const markdown = await response.text();
      if (/^\s*</.test(markdown) && /<html[\s>]/i.test(markdown)) {
        throw new Error('received HTML instead of markdown');
      }
      if (typeof marked === 'undefined') throw new Error('markdown renderer missing');
      contentEl.innerHTML = marked.parse(markdown);
      rewriteMarkdownLinks(contentEl, docPath);
      const firstHeading = contentEl.querySelector('h1');
      const title = firstHeading ? firstHeading.textContent : docPath;
      if (titleEl) titleEl.textContent = title;
      document.title = `${title} · QualiaDB`;
      buildToc(contentEl, tocEl);
      if (window.location.hash) {
        const target = document.getElementById(window.location.hash.slice(1));
        if (target) target.scrollIntoView();
      }
    } catch (error) {
      contentEl.innerHTML = `<p>Could not load <code class="q-mono">${docPath}</code> (${error.message}). The source markdown is still on the branch.</p>`;
      if (titleEl) titleEl.textContent = 'Could not load document';
    }
  }

  function rewriteStandalonePage() {
    const current = inferCurrentDocFromLocation();
    const root = document.querySelector('.doc-content, .q-prose, main');
    if (!current || !root) return;
    rewriteMarkdownLinks(root, current);
  }

  window.QualiaMarkdownDoc = {
    normalizeDocPath,
    resolveRelativeDoc,
    rewriteMarkdownLinks,
    readerHref,
  };

  const mode = document.currentScript?.dataset.mode || 'auto';
  const start = () => {
    if (mode === 'rewrite' || (mode === 'auto' && !document.getElementById('doc-content'))) {
      rewriteStandalonePage();
      return;
    }
    renderReader();
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start);
  } else {
    start();
  }
})();
